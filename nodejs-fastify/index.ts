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

      // First pass: Set up httpGet to collect HTTP requests
      vm.unwrapResult(vm.evalCode(`
        var __httpRequests = [];
        function httpGet(url, options) {
          __httpRequests.push({ url: url, options: options || {} });
          return undefined;
        }
      `));

      // First execution to discover HTTP calls
      let httpRequests: Array<{ url: string; options: any }> = [];
      try {
        const firstResult = vm.unwrapResult(vm.evalCode(code));
        const firstOutput = vm.dump(firstResult);
        firstResult.dispose();
        
        // Extract HTTP requests
        const requestsJson = vm.unwrapResult(vm.evalCode('JSON.stringify(__httpRequests)'));
        const requestsStr = vm.getString(requestsJson);
        requestsJson.dispose();
        httpRequests = JSON.parse(requestsStr);
        
        // If no HTTP requests were made, return the result immediately
        if (httpRequests.length === 0) {
          vm.dispose();
          return reply.send({ result: firstOutput });
        }
      } catch (e: any) {
        // Expected to fail if code depends on HTTP results
        request.log.debug(`First pass error (expected): ${e.message}`);
        
        // Still extract HTTP requests
        const requestsJson = vm.unwrapResult(vm.evalCode('JSON.stringify(__httpRequests)'));
        const requestsStr = vm.getString(requestsJson);
        requestsJson.dispose();
        httpRequests = JSON.parse(requestsStr);
      }

      request.log.debug(`Found ${httpRequests.length} HTTP requests to execute`);

      // Execute all discovered HTTP requests
      const results = await Promise.all(
        httpRequests.map(({ url, options }) => performFetch(url, options))
      );

      // Build results map
      const httpResults: Record<string, any> = {};
      httpRequests.forEach((req, index) => {
        const key = JSON.stringify({ url: req.url, options: req.options });
        httpResults[key] = results[index];
      });

      // Create a fresh context with HTTP results
      vm.dispose();
      const QuickJS2 = await getQuickJS();
      const vm2 = QuickJS2.newContext();

      // Re-inject INPUTS
      injectObject(vm2, 'INPUTS', inputs);

      // Inject HTTP results and set up httpGet to return cached results
      injectObject(vm2, '__httpResults', httpResults);
      vm2.unwrapResult(vm2.evalCode(`
        function httpGet(url, options) {
          var key = JSON.stringify({ url: url, options: options || {} });
          return __httpResults[key];
        }
      `));

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
