'use strict';

/**
 * Test suite for edge-agents Cloud Function.
 *
 * Validates: entry point, routing, CORS, contracts, health,
 * execution_metadata, layers_executed, and all 5 agent handlers.
 *
 * Run: node test/cloud-functions.test.js
 */

const assert = require('assert');
const crypto = require('crypto');

// ─── test harness ───────────────────────────────────────────────────────────

let passed = 0;
let failed = 0;
const failures = [];

async function test(name, fn) {
  try {
    await fn();
    passed++;
    console.log(`  \x1b[32m✓\x1b[0m ${name}`);
  } catch (err) {
    failed++;
    failures.push({ name, error: err.message });
    console.log(`  \x1b[31m✗\x1b[0m ${name}`);
    console.log(`    ${err.message}`);
  }
}

function mockReq(method, path, body, headers) {
  return {
    method,
    path,
    url: path,
    body: body || {},
    headers: headers || {},
  };
}

function mockRes() {
  const res = {
    _status: null,
    _body: null,
    _headers: {},
    status(code) { res._status = code; return res; },
    json(body)   { res._body = body; return res; },
    send(body)   { res._body = body; return res; },
    set(k, v)    { res._headers[k] = v; return res; },
  };
  return res;
}

// ─── modules under test ─────────────────────────────────────────────────────

const { handler } = require('../index');
const { route, ROUTE_TABLE, AGENTS } = require('../cloud-functions/router');
const { setCorsHeaders, handlePreflight } = require('../cloud-functions/middleware');
const contracts = require('../cloud-functions/contracts');

// ─── test suites ────────────────────────────────────────────────────────────

async function run() {
  console.log('\n\x1b[1medge-agents Cloud Function tests\x1b[0m\n');

  // ── Entry point ───────────────────────────────────────────────────────────

  console.log('Entry point:');

  await test('handler is exported as a function', () => {
    assert.strictEqual(typeof handler, 'function');
  });

  // ── CORS ──────────────────────────────────────────────────────────────────

  console.log('\nCORS:');

  await test('OPTIONS returns 204 with CORS headers', async () => {
    const req = mockReq('OPTIONS', '/v1/edge/tool-invoke');
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 204);
    assert.ok(res._headers['Access-Control-Allow-Origin']);
    assert.ok(res._headers['Access-Control-Allow-Methods']);
    assert.ok(res._headers['Access-Control-Allow-Headers']);
  });

  await test('non-OPTIONS responses include CORS headers', async () => {
    const req = mockReq('GET', '/health');
    const res = mockRes();
    await handler(req, res);
    assert.ok(res._headers['Access-Control-Allow-Origin']);
  });

  // ── Health ────────────────────────────────────────────────────────────────

  console.log('\nHealth:');

  await test('GET /health returns 200 with all 5 agents', async () => {
    const req = mockReq('GET', '/health');
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.status, 'healthy');
    assert.strictEqual(res._body.service, 'edge-agents');
    const agentNames = Object.keys(res._body.agents);
    assert.deepStrictEqual(agentNames.sort(), [
      'caching', 'circuit-breaker', 'execution-guard', 'failover', 'tool-invoke',
    ]);
  });

  await test('GET / aliases to health', async () => {
    const req = mockReq('GET', '/');
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.status, 'healthy');
  });

  // ── execution_metadata ────────────────────────────────────────────────────

  console.log('\nexecution_metadata:');

  await test('response includes execution_metadata with required fields', async () => {
    const req = mockReq('GET', '/health');
    const res = mockRes();
    await handler(req, res);
    const em = res._body.execution_metadata;
    assert.ok(em, 'execution_metadata missing');
    assert.ok(em.trace_id, 'trace_id missing');
    assert.ok(em.timestamp, 'timestamp missing');
    assert.strictEqual(em.service, 'edge-agents');
    assert.ok(em.execution_id, 'execution_id missing');
  });

  await test('x-correlation-id header is used as trace_id', async () => {
    const correlationId = 'test-trace-' + crypto.randomUUID();
    const req = mockReq('GET', '/health', null, { 'x-correlation-id': correlationId });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._body.execution_metadata.trace_id, correlationId);
  });

  // ── layers_executed ───────────────────────────────────────────────────────

  console.log('\nlayers_executed:');

  await test('response includes layers_executed with AGENT_ROUTING', async () => {
    const req = mockReq('GET', '/health');
    const res = mockRes();
    await handler(req, res);
    const layers = res._body.layers_executed;
    assert.ok(Array.isArray(layers), 'layers_executed must be an array');
    assert.strictEqual(layers[0].layer, 'AGENT_ROUTING');
    assert.strictEqual(layers[0].status, 'completed');
  });

  await test('layers_executed includes agent-specific layer with duration_ms', async () => {
    const req = mockReq('POST', '/v1/edge/tool-invoke', {
      tool: { name: 'test-tool' },
      arguments: {},
    });
    const res = mockRes();
    await handler(req, res);
    const layers = res._body.layers_executed;
    assert.strictEqual(layers[1].layer, 'EDGE_TOOL_INVOKE');
    assert.strictEqual(typeof layers[1].duration_ms, 'number');
  });

  // ── Routing ───────────────────────────────────────────────────────────────

  console.log('\nRouting:');

  await test('all 5 routes are registered', () => {
    const routes = Object.keys(ROUTE_TABLE);
    assert.ok(routes.includes('/v1/edge/tool-invoke'));
    assert.ok(routes.includes('/v1/edge/circuit-breaker'));
    assert.ok(routes.includes('/v1/edge/failover'));
    assert.ok(routes.includes('/v1/edge/execution-guard'));
    assert.ok(routes.includes('/v1/edge/caching'));
  });

  await test('unknown route returns 404', async () => {
    const req = mockReq('GET', '/v1/edge/nonexistent');
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 404);
    assert.ok(res._body.error);
  });

  // ── Contracts endpoint ────────────────────────────────────────────────────

  console.log('\nContracts:');

  await test('GET /v1/edge/contracts returns all 5 schemas', async () => {
    const req = mockReq('GET', '/v1/edge/contracts');
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    const agents = res._body.agents;
    assert.ok(agents['tool-invoke']);
    assert.ok(agents['circuit-breaker']);
    assert.ok(agents['failover']);
    assert.ok(agents['execution-guard']);
    assert.ok(agents['caching']);
  });

  // ── Tool Invocation Agent ─────────────────────────────────────────────────

  console.log('\nTool Invocation Agent:');

  await test('POST valid request returns allow', async () => {
    const req = mockReq('POST', '/v1/edge/tool-invoke', {
      tool: { name: 'calculator' },
      arguments: { expression: '2+2' },
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.success, true);
    assert.strictEqual(res._body.data.decision, 'allow');
  });

  await test('POST blocked tool returns 403', async () => {
    const req = mockReq('POST', '/v1/edge/tool-invoke', {
      tool: { name: 'dangerous-tool' },
      arguments: {},
      constraints: { blocked_tools: ['dangerous-tool'] },
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 403);
    assert.strictEqual(res._body.error.code, 'TOOL_BLOCKED');
  });

  await test('POST missing tool field returns 400', async () => {
    const req = mockReq('POST', '/v1/edge/tool-invoke', { arguments: {} });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 400);
  });

  await test('GET returns inspect metadata', async () => {
    const req = mockReq('GET', '/v1/edge/tool-invoke');
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.agent_id, 'tool-invocation-agent');
  });

  // ── Circuit Breaker Agent ─────────────────────────────────────────────────

  console.log('\nCircuit Breaker Agent:');

  await test('POST closed circuit returns allow', async () => {
    const req = mockReq('POST', '/v1/edge/circuit-breaker', {
      provider_id: 'openai',
      execution_ref: 'test-123',
      current_state: 'closed',
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.decision.action, 'allow');
    assert.strictEqual(res._body.data.new_state, 'closed');
  });

  await test('POST open circuit returns block', async () => {
    const req = mockReq('POST', '/v1/edge/circuit-breaker', {
      provider_id: 'openai',
      execution_ref: 'test-456',
      current_state: 'open',
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.decision.action, 'block');
  });

  await test('POST failures trigger state transition to open', async () => {
    const req = mockReq('POST', '/v1/edge/circuit-breaker', {
      provider_id: 'openai',
      execution_ref: 'test-789',
      current_state: 'closed',
      failure_count: 4,
      failure_signal: { provider_id: 'openai', failure_type: 'timeout' },
      config: { failure_threshold: 5 },
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.new_state, 'open');
    assert.ok(res._body.data.state_transition);
  });

  // ── Failover Agent ────────────────────────────────────────────────────────

  console.log('\nFailover Agent:');

  await test('POST healthy primary returns use_primary', async () => {
    const req = mockReq('POST', '/v1/edge/failover', {
      execution_context: { execution_ref: 'test-123' },
      primary_target: {
        provider_id: 'openai-1',
        provider_name: 'openai',
        model: 'gpt-4',
        circuit_state: 'closed',
        health: 'healthy',
      },
      alternate_targets: [],
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.decision.outcome, 'use_primary');
  });

  await test('POST unhealthy primary fails over to alternate', async () => {
    const req = mockReq('POST', '/v1/edge/failover', {
      execution_context: { execution_ref: 'test-456' },
      primary_target: {
        provider_id: 'openai-1',
        provider_name: 'openai',
        model: 'gpt-4',
        circuit_state: 'open',
        health: 'unhealthy',
      },
      alternate_targets: [{
        provider_id: 'anthropic-1',
        provider_name: 'anthropic',
        model: 'claude-3',
        circuit_state: 'closed',
        health: 'healthy',
        priority: 1,
      }],
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.decision.outcome, 'failover');
    assert.strictEqual(res._body.data.decision.selected_target.provider_name, 'anthropic');
  });

  await test('POST all targets down returns no_available_target', async () => {
    const req = mockReq('POST', '/v1/edge/failover', {
      execution_context: { execution_ref: 'test-789' },
      primary_target: {
        provider_id: 'p1', provider_name: 'p1', model: 'm',
        circuit_state: 'open', health: 'unhealthy',
      },
      alternate_targets: [{
        provider_id: 'a1', provider_name: 'a1', model: 'm',
        circuit_state: 'open', health: 'unhealthy', priority: 1,
      }],
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._body.data.decision.outcome, 'no_available_target');
  });

  // ── Execution Guard Agent ─────────────────────────────────────────────────

  console.log('\nExecution Guard Agent:');

  await test('POST within limits returns allow', async () => {
    const req = mockReq('POST', '/v1/edge/execution-guard', {
      envelope: {
        request_id: 'req-1',
        model: 'gpt-4',
        input_tokens: 1000,
        max_output_tokens: 2000,
        current_concurrency: 5,
      },
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.decision, 'allow');
  });

  await test('POST exceeding token limit returns block', async () => {
    const req = mockReq('POST', '/v1/edge/execution-guard', {
      envelope: {
        request_id: 'req-2',
        model: 'gpt-4',
        input_tokens: 200000,
        max_output_tokens: 2000,
        current_concurrency: 5,
      },
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 403);
    assert.strictEqual(res._body.data.decision, 'block');
  });

  await test('POST blocked model returns block', async () => {
    const req = mockReq('POST', '/v1/edge/execution-guard', {
      envelope: {
        request_id: 'req-3',
        model: 'dangerous-model',
        input_tokens: 100,
        max_output_tokens: 100,
        current_concurrency: 1,
      },
      limits: { blocked_models: ['dangerous-model'] },
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 403);
    assert.ok(res._body.data.violations.some(v => v.code === 'MODEL_BLOCKED'));
  });

  // ── Caching Strategy Agent ────────────────────────────────────────────────

  console.log('\nCaching Strategy Agent:');

  await test('POST with cache hit returns cache_hit', async () => {
    const req = mockReq('POST', '/v1/edge/caching', {
      input: {
        execution_ref: 'exec-1',
        request_type: 'chat_completion',
        content_hash: 'hash-1',
        prompt_hash: 'prompt-1',
        cache_relevant_params: { temperature: 0.0 },
        check_cache: true,
        is_write_operation: false,
        existing_entry: {
          cache_key: 'cache:hash-1',
          layer: 'L1',
          is_valid: true,
          is_stale: false,
          access_count: 3,
        },
      },
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.decision, 'cache_hit');
  });

  await test('POST with no cache entry returns cache_miss', async () => {
    const req = mockReq('POST', '/v1/edge/caching', {
      input: {
        execution_ref: 'exec-2',
        request_type: 'chat_completion',
        content_hash: 'hash-2',
        prompt_hash: 'prompt-2',
        cache_relevant_params: { temperature: 0.0 },
        check_cache: true,
        is_write_operation: false,
      },
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.decision, 'cache_miss');
  });

  await test('POST streaming request returns cache_bypass', async () => {
    const req = mockReq('POST', '/v1/edge/caching', {
      input: {
        execution_ref: 'exec-3',
        request_type: 'chat_completion',
        content_hash: 'hash-3',
        prompt_hash: 'prompt-3',
        cache_relevant_params: { stream: true },
        check_cache: true,
        is_write_operation: false,
      },
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.decision, 'cache_bypass');
  });

  await test('POST write operation returns cache_write', async () => {
    const req = mockReq('POST', '/v1/edge/caching', {
      input: {
        execution_ref: 'exec-4',
        request_type: 'chat_completion',
        content_hash: 'hash-4',
        prompt_hash: 'prompt-4',
        cache_relevant_params: { temperature: 0.0 },
        check_cache: false,
        is_write_operation: true,
        response_metadata: {
          response_size_bytes: 500,
          latency_ms: 200,
          success: true,
          contains_sensitive_data: false,
        },
      },
    });
    const res = mockRes();
    await handler(req, res);
    assert.strictEqual(res._status, 200);
    assert.strictEqual(res._body.data.decision, 'cache_write');
  });

  // ── Error handling ────────────────────────────────────────────────────────

  console.log('\nError handling:');

  await test('404 response includes execution_metadata', async () => {
    const req = mockReq('GET', '/v1/edge/nonexistent');
    const res = mockRes();
    await handler(req, res);
    assert.ok(res._body.execution_metadata);
    assert.ok(res._body.layers_executed);
  });

  // ── Summary ───────────────────────────────────────────────────────────────

  console.log(`\n\x1b[1mResults: ${passed} passed, ${failed} failed\x1b[0m`);
  if (failures.length > 0) {
    console.log('\nFailures:');
    for (const f of failures) {
      console.log(`  - ${f.name}: ${f.error}`);
    }
  }
  console.log('');

  process.exit(failed > 0 ? 1 : 0);
}

run().catch(err => {
  console.error('Test runner error:', err);
  process.exit(1);
});
