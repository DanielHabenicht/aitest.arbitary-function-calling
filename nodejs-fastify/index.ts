import Fastify from 'fastify';
import { getQuickJS, shouldInterruptAfterDeadline } from 'quickjs-emscripten';

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

  try {
    // Create QuickJS VM
    const QuickJS = await getQuickJS();
    const vm = QuickJS.newContext();

    // Set execution timeout (10 seconds)
    const interruptCycleLimit = 1024;
    let interruptCycles = 0;
    const shouldInterrupt = shouldInterruptAfterDeadline(Date.now() + 10000);
    vm.runtime.setInterruptHandler((runtime) => {
      interruptCycles++;
      if (interruptCycles % interruptCycleLimit === 0) {
        return shouldInterrupt(runtime);
      }
      return false;
    });

    try {
      // Inject INPUTS as a global variable
      injectObject(vm, 'INPUTS', inputs);

      // Provide a simple httpGet function that returns data synchronously
      // It works by taking a URL and returning the fetched data
      // The user should call it like: httpGet(url) or httpGet(url, options)
      // On first execution, we collect all URLs and execute them, then re-run the code
      const httpRequests: Array<{ url: string; options: any }> = [];
      const httpResults = new Map<string, any>();

      const httpGetFn = vm.newFunction('httpGet', (urlHandle, optionsHandle) => {
        const url = vm.getString(urlHandle);
        let options: any = {};
        
        if (optionsHandle) {
          const optionsJson = vm.dump(optionsHandle);
          options = optionsJson;
        }

        const key = JSON.stringify({ url, options });
        
        // If we have a result, return it
        if (httpResults.has(key)) {
          const result = httpResults.get(key);
          return vm.unwrapResult(vm.evalCode(`(${JSON.stringify(result)})`));
        }
        
        // Otherwise, record this request
        if (!httpRequests.some(r => JSON.stringify({ url: r.url, options: r.options }) === key)) {
          httpRequests.push({ url, options });
        }
        
        // Return undefined for first pass
        return vm.undefined;
      });
      
      vm.setProp(vm.global, 'httpGet', httpGetFn);
      httpGetFn.dispose();

      // First execution to discover HTTP calls
      try {
        const firstResult = vm.unwrapResult(vm.evalCode(code));
        const firstOutput = vm.dump(firstResult);
        firstResult.dispose();
        
        // If no HTTP requests were made, return the result immediately
        if (httpRequests.length === 0) {
          vm.dispose();
          return reply.send({ result: firstOutput });
        }
      } catch (e: any) {
        // Expected to fail if code depends on HTTP results
        request.log.debug(`First pass error (expected): ${e.message}`);
      }

      request.log.debug(`Found ${httpRequests.length} HTTP requests to execute`);

      // Execute all discovered HTTP requests
      const results = await Promise.all(
        httpRequests.map(({ url, options }) => performFetch(url, options))
      );

      // Store results in map
      httpRequests.forEach((req, index) => {
        const key = JSON.stringify({ url: req.url, options: req.options });
        httpResults.set(key, results[index]);
      });

      // Create a fresh context with HTTP results
      vm.dispose();
      const QuickJS2 = await getQuickJS();
      const vm2 = QuickJS2.newContext();

      // Re-inject INPUTS
      injectObject(vm2, 'INPUTS', inputs);

      // Create httpGet that returns actual results
      const httpGetFinalFn = vm2.newFunction('httpGet', (urlHandle, optionsHandle) => {
        const url = vm2.getString(urlHandle);
        let options: any = {};
        
        if (optionsHandle) {
          const optionsJson = vm2.dump(optionsHandle);
          options = optionsJson;
        }

        const key = JSON.stringify({ url, options });
        const result = httpResults.get(key);
        
        if (result) {
          return vm2.unwrapResult(vm2.evalCode(`(${JSON.stringify(result)})`));
        }
        
        return vm2.undefined;
      });
      
      vm2.setProp(vm2.global, 'httpGet', httpGetFinalFn);
      httpGetFinalFn.dispose();

      // Final execution with actual HTTP results
      const result = vm2.unwrapResult(vm2.evalCode(code));
      const output = vm2.dump(result);
      result.dispose();

      vm2.dispose();
      return reply.send({ result: output });
    } catch (error: any) {
      // Make sure to clean up VMs if they exist
      try { vm.dispose(); } catch (e) {}
      throw error;
    }
  } catch (error: any) {
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
