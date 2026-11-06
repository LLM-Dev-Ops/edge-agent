'use strict';
const assert = require('node:assert/strict');
const { createRequire } = require('node:module');
const { resolve, join } = require('node:path');
const { randomUUID } = require('node:crypto');
const contract = require('./contract.json');
const guard = extra => ({ envelope: { request_id: 'synthetic-1', model: 'example-model', input_tokens: 10, max_output_tokens: 20, current_concurrency: 0, ...extra } });
const target = { provider_id: 'example-1', provider_name: 'example', model: 'example-model', circuit_state: 'closed', health: 'healthy' };
const specs = {
  'guard-envelope-string': { body: { envelope: 'invalid' } },
  'guard-missing-model': { body: { envelope: { request_id: 'r', input_tokens: 1, max_output_tokens: 1 } } },
  'guard-string-tokens': { body: guard({ input_tokens: 'not-a-number' }) },
  'guard-negative-tokens': { body: guard({ input_tokens: -1 }) },
  'guard-fractional-tokens': { body: guard({ max_output_tokens: 0.5 }) },
  'guard-string-cost': { body: guard({ estimated_cost_micros: 'not-a-number' }) },
  'guard-nonfinite-cost': { raw: '{"envelope":{"request_id":"r","model":"m","input_tokens":1,"max_output_tokens":1,"estimated_cost_micros":1e400}}' },
  'guard-string-concurrency': { body: guard({ current_concurrency: 'not-a-number' }) },
  'guard-negative-rate': { body: { ...guard(), current_rate: -1 } },
  'guard-string-limit': { body: { ...guard(), limits: { max_input_tokens: 'not-a-number' } } },
  'guard-null-limit': { body: { ...guard(), limits: { max_concurrency: null } } },
  'guard-string-blocklist': { body: { ...guard(), limits: { blocked_models: 'different-model' } } },
  'guard-invalid-allowlist-item': { body: { ...guard(), limits: { allowed_models: [123] } } },
  'tool-numeric-name': { path: 'tool-invoke', body: { tool: { name: 42 }, arguments: {} } },
  'tool-array-arguments': { path: 'tool-invoke', body: { tool: { name: 'calculator' }, arguments: [] } },
  'circuit-invalid-state': { path: 'circuit-breaker', body: { provider_id: 'example', execution_ref: 'r', current_state: 'invalid' } },
  'failover-missing-model': { path: 'failover', body: { execution_context: {}, primary_target: { provider_id: 'example', provider_name: 'example' } } },
  'failover-string-alternates': { path: 'failover', body: { execution_context: {}, primary_target: target, alternate_targets: 'not-an-array' } },
  'cache-string-input': { path: 'caching', body: { input: 'invalid' } },
  'cache-string-stream': { path: 'caching', body: { input: { execution_ref: 'r', request_type: 'chat_completion', content_hash: 'c', prompt_hash: 'p', cache_relevant_params: { stream: 'false' } } } },
  'guard-default-concurrency': { body: { envelope: { request_id: 'r', model: 'example-model', input_tokens: 1, max_output_tokens: 1 } }, status: 200, check: b => { assert.equal(b.data.envelope_summary.current_concurrency, 0); assert(b.data.constraints_evaluated.every(x => x.passed)); } },
  'guard-allow': { body: guard(), status: 200, decision: 'allow' },
  'guard-token-block': { body: guard({ input_tokens: 100001 }), status: 403, decision: 'block' },
  'guard-cost-block': { body: guard({ estimated_cost_micros: 500001 }), status: 403, decision: 'block' },
  'guard-concurrency-block': { body: guard({ current_concurrency: 51 }), status: 403, decision: 'block' },
  'guard-rate-block': { body: { ...guard(), current_rate: 1001 }, status: 403, decision: 'block' },
  'guard-model-block': { body: { ...guard(), limits: { blocked_models: ['example-model'] } }, status: 403, decision: 'block' },
  'tool-allow': { path: 'tool-invoke', body: { tool: { name: 'calculator' }, arguments: {} }, status: 200, decision: 'allow' },
  'tool-block': { path: 'tool-invoke', body: { tool: { name: 'blocked' }, arguments: {}, constraints: { blocked_tools: ['blocked'] } }, status: 403, decision: 'block' },
  'circuit-closed': { path: 'circuit-breaker', body: { provider_id: 'example', execution_ref: 'r', current_state: 'closed' }, status: 200, check: b => assert.equal(b.data.decision.action, 'allow') },
  'failover-primary': { path: 'failover', body: { execution_context: {}, primary_target: target, alternate_targets: [] }, status: 200, check: b => assert.equal(b.data.decision.outcome, 'use_primary') },
  'cache-miss': { path: 'caching', body: { input: { execution_ref: 'r', request_type: 'chat_completion', content_hash: 'c', prompt_hash: 'p', cache_relevant_params: { temperature: 0 }, check_cache: true } }, status: 200, decision: 'cache_miss' },
  'health': { url: '/health', method: 'GET', status: 200, check: b => assert.equal(b.status, 'healthy') },
};
async function probeContract(root, names = [...contract.holdout, ...contract.anchor]) {
  const local = createRequire(join(resolve(root), 'package.json'));
  const framework = local('@google-cloud/functions-framework');
  const { getTestServer } = local('@google-cloud/functions-framework/testing');
  const name = 'edge-contract-' + randomUUID();
  framework.http(name, local('./index.js').handler);
  const server = getTestServer(name);
  await new Promise((ok, bad) => { server.once('error', bad); server.listen(0, '127.0.0.1', ok); });
  const base = 'http://127.0.0.1:' + server.address().port;
  const results = [];
  try {
    for (const name of names) {
      const spec = specs[name], start = performance.now(); let status;
      try {
        assert(spec, 'Unknown frozen case');
        const response = await fetch(base + (spec.url || '/v1/edge/' + (spec.path || 'execution-guard')), {
          method: spec.method || 'POST', headers: { 'Content-Type': 'application/json' },
          body: spec.method === 'GET' ? undefined : spec.raw || JSON.stringify(spec.body), signal: AbortSignal.timeout(3000),
        });
        status = response.status; const body = await response.json();
        assert.equal(status, spec.status || 400);
        if (!spec.status) assert.equal(body.error.code, 'VALIDATION_ERROR');
        if (spec.decision) assert.equal(body.data.decision, spec.decision);
        if (spec.check) spec.check(body);
        assert(body.execution_metadata.trace_id);
        results.push({ name, pass: true, status, ms: performance.now() - start });
      } catch (e) { results.push({ name, pass: false, status, error: String(e), ms: performance.now() - start }); }
    }
  } finally { server.closeAllConnections(); await new Promise(ok => server.close(ok)); }
  return results;
}
module.exports = { contract, probeContract };
if (require.main === module) probeContract(process.argv[2]).then(r => console.log(JSON.stringify(r, null, 2))).catch(e => { console.error(e); process.exitCode = 1; });
