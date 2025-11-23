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

  // Create async context
  const context = await newAsyncContext();

  try {
    // Inject INPUTS as a global variable
    injectObject(context, 'INPUTS', inputs);

    // Create a native async function for httpGet
    const httpGetFn = context.newAsyncifiedFunction('httpGet', async (urlHandle, optionsHandle) => {
      const url = context.getString(urlHandle);
      let options: any = {};
      
      if (optionsHandle) {
        const optionsJson = context.dump(optionsHandle);
        options = optionsJson;
      }

      // Perform the actual fetch
      const result = await performFetch(url, options);
      
      // Return result to QuickJS
      const resultHandle = context.unwrapResult(
        context.evalCode(`(${JSON.stringify(result)})`)
      );
      return resultHandle;
    });

    context.setProp(context.global, 'httpGet', httpGetFn);
    httpGetFn.dispose();

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
