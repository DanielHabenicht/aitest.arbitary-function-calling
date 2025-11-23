import Fastify from 'fastify';
import { newAsyncContext } from 'quickjs-emscripten';

const fastify = Fastify({
  logger: true
});

interface ExecuteRequest {
  code: string;
  inputs: Record<string, any>;
}

// Helper to inject JavaScript object into QuickJS VM
function injectObject(vm: any, name: string, obj: any) {
  const handle = vm.unwrapResult(vm.evalCode(`(${JSON.stringify(obj)})`));
  vm.setProp(vm.global, name, handle);
  handle.dispose();
}

// Helper to perform fetch and return serialized result
async function performFetch(url: string, options: any = {}): Promise<any> {
  try {
    const response = await fetch(url, {
      method: options.method || 'GET',
      headers: options.headers || {},
      body: options.body || undefined,
    });

    const contentType = response.headers.get('content-type') || '';
    let data: any;

    if (contentType.includes('application/json')) {
      data = await response.json();
    } else {
      data = await response.text();
    }

    return {
      ok: response.ok,
      status: response.status,
      statusText: response.statusText,
      headers: Object.fromEntries(response.headers.entries()),
      data: data
    };
  } catch (error: any) {
    return {
      ok: false,
      status: 0,
      statusText: 'Error',
      headers: {},
      error: error.message
    };
  }
}

// POST /execute endpoint
fastify.post<{ Body: ExecuteRequest }>('/execute', async (request, reply) => {
  const { code, inputs } = request.body;

  if (!code || typeof code !== 'string') {
    return reply.code(400).send({ error: 'Invalid code parameter' });
  }

  // Create async context with timeout
  const context = await newAsyncContext();

  try {
    // Set execution timeout (10 seconds)
    const timeoutMs = 10000;
    const startTime = Date.now();
    context.runtime.setInterruptHandler(() => {
      return Date.now() - startTime > timeoutMs;
    });

    // Inject INPUTS as a global variable
    injectObject(context, 'INPUTS', inputs);

    // Create a native async function for HTTP requests (supports all verbs)
    const httpRequestFn = context.newAsyncifiedFunction('httpRequest', async (urlHandle, optionsHandle) => {
      const url = context.getString(urlHandle);
      let options: any = {};
      
      if (optionsHandle) {
        const optionsJson = context.dump(optionsHandle);
        options = optionsJson;
      }

      // Perform the actual fetch with any HTTP method
      const result = await performFetch(url, options);
      
      // Return result to QuickJS using safe injection method
      // Store in a temporary global variable to avoid code injection
      const tempVarName = `__httpResult_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
      const globalObj = context.global;
      const resultStr = JSON.stringify(result);
      
      // Use evalCode with the JSON string but avoid injection by parsing it
      const parsedHandle = context.unwrapResult(
        context.evalCode(`JSON.parse(${JSON.stringify(resultStr)})`)
      );
      return parsedHandle;
    });

    context.setProp(context.global, 'httpRequest', httpRequestFn);
    httpRequestFn.dispose();

    // Also provide httpGet as a convenience wrapper for backwards compatibility
    context.unwrapResult(context.evalCode(`
      function httpGet(url, options) {
        return httpRequest(url, options);
      }
    `)).dispose();

    // Execute the user code asynchronously
    const resultPromise = await context.evalCodeAsync(code);
    const resultHandle = context.unwrapResult(resultPromise);
    
    const output = context.dump(resultHandle);
    resultHandle.dispose();

    context.dispose();

    return reply.send({ result: output });
  } catch (error: any) {
    // Clean up resources
    try { context.dispose(); } catch (e) {}
    
    console.error(error);
    return reply.code(500).send({ 
      error: 'Execution failed', 
      message: error.message 
    });
  }
});

// Health check endpoint
fastify.get('/health', async (request, reply) => {
  return { status: 'ok' };
});

// Start server
const start = async () => {
  try {
    const port = process.env.PORT ? parseInt(process.env.PORT) : 3000;
    const host = process.env.HOST || '0.0.0.0';
    
    await fastify.listen({ port, host });
    console.log(`Server listening on ${host}:${port}`);
  } catch (err) {
    fastify.log.error(err);
    process.exit(1);
  }
};

start();
