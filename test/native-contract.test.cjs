'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { contract, probeContract } = require('../security/probe-contract.cjs');
const { validateRequest, executionGuardContract } = require('../cloud-functions/contracts');

test('native Functions Framework frozen contract', async t => {
  const results = await probeContract(require('node:path').resolve(__dirname, '..'));
  for (const result of results) await t.test(result.name, () => assert.equal(result.pass, true, result.error));
  assert.equal(results.length, contract.holdout.length + contract.anchor.length);
});

test('required envelope fields must be own properties', () => {
  const envelope = Object.create({ request_id: 'inherited', model: 'example', input_tokens: 1, max_output_tokens: 1 });
  assert.equal(validateRequest({ envelope }, executionGuardContract.request).valid, false);
});

test('integer validation rejects unsafe values and nonfinite values without coercion', () => {
  for (const input_tokens of [Number.MAX_SAFE_INTEGER + 1, Infinity, NaN, true, [], null]) {
    const body = { envelope: { request_id: 'r', model: 'example', input_tokens, max_output_tokens: 1 } };
    assert.equal(validateRequest(body, executionGuardContract.request).valid, false);
  }
});

test('validation work and diagnostic counts are bounded', () => {
  const body = { envelope: { request_id: 'r', model: 'example', input_tokens: 1, max_output_tokens: 1 }, limits: { allowed_models: Array(20000).fill(42) } };
  const result = validateRequest(body, executionGuardContract.request);
  assert.equal(result.valid, false); assert(result.errors.length <= 32);
});

test('valid opaque arguments and optional unknown properties are preserved', () => {
  const { toolInvokeContract } = require('../cloud-functions/contracts');
  const body = { tool: { name: 'example' }, arguments: { nested: { anything: [1, null] } }, future_field: true };
  assert.equal(validateRequest(body, toolInvokeContract.request).valid, true);
});
