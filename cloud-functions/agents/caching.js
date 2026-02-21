'use strict';

const { cachingContract, validateRequest } = require('../contracts');

const AGENT_ID = 'caching-strategy-agent';
const AGENT_VERSION = '0.1.0';
const CLASSIFICATION = 'EXECUTION CONTROL';

/**
 * Caching Strategy Agent handler.
 *
 * Evaluates cache eligibility and returns cache directives
 * (hit, miss, write, bypass, invalidate).
 *
 * POST /v1/edge/caching
 */
async function handle(req) {
  if (req.method === 'GET') {
    return inspect();
  }

  if (req.method !== 'POST') {
    return {
      status: 405,
      body: { error: { code: 'METHOD_NOT_ALLOWED', message: 'Use POST for invocations, GET for inspect' } },
      layerStatus: 'failed',
    };
  }

  const body = req.body;
  const validation = validateRequest(body, cachingContract.request);
  if (!validation.valid) {
    return {
      status: 400,
      body: {
        success: false,
        error: { code: 'VALIDATION_ERROR', message: validation.errors.join('; ') },
      },
      layerStatus: 'completed',
    };
  }

  const input = body.input;
  const params = input.cache_relevant_params || {};
  const constraintsEvaluated = [];

  // Determine if request is cacheable
  const isStreamRequest = params.stream === true;
  const hasHighTemperature = (params.temperature || 0) > 0.5;
  const hasSeed = params.seed != null;

  constraintsEvaluated.push({
    constraint_type: 'cache',
    name: 'stream_check',
    passed: !isStreamRequest,
    details: isStreamRequest ? 'Streaming requests are not cacheable' : 'Non-streaming request',
  });

  constraintsEvaluated.push({
    constraint_type: 'cache',
    name: 'temperature_check',
    passed: !hasHighTemperature || hasSeed,
    details: hasHighTemperature
      ? (hasSeed ? 'High temperature but deterministic seed provided' : 'High temperature — non-deterministic')
      : 'Temperature <= 0.5 — deterministic',
  });

  // Bypass if streaming or non-deterministic high temperature
  if (isStreamRequest || (hasHighTemperature && !hasSeed)) {
    return {
      status: 200,
      body: {
        success: true,
        data: {
          agent_id: AGENT_ID,
          agent_version: AGENT_VERSION,
          decision: 'cache_bypass',
          directives: [{ action: 'bypass', reason: isStreamRequest ? 'streaming' : 'non_deterministic' }],
          confidence: 0.95,
          constraints_evaluated: constraintsEvaluated,
          classification: CLASSIFICATION,
        },
      },
    };
  }

  // Check for existing cache entry
  if (input.check_cache && input.existing_entry) {
    const entry = input.existing_entry;
    if (entry.is_valid && !entry.is_stale) {
      constraintsEvaluated.push({
        constraint_type: 'cache',
        name: 'entry_validity',
        passed: true,
        details: `Valid cache entry found in ${entry.layer || 'L1'} (hits: ${entry.access_count || 0})`,
      });

      return {
        status: 200,
        body: {
          success: true,
          data: {
            agent_id: AGENT_ID,
            agent_version: AGENT_VERSION,
            decision: 'cache_hit',
            directives: [
              { action: 'serve_cached', cache_key: entry.cache_key, layer: entry.layer || 'L1' },
            ],
            confidence: 1.0,
            constraints_evaluated: constraintsEvaluated,
            classification: CLASSIFICATION,
          },
        },
      };
    }

    if (entry.is_stale) {
      constraintsEvaluated.push({
        constraint_type: 'cache',
        name: 'entry_staleness',
        passed: false,
        details: 'Cache entry is stale — needs revalidation',
      });
    }
  }

  // Write operation
  if (input.is_write_operation && input.response_metadata) {
    const meta = input.response_metadata;

    if (meta.contains_sensitive_data) {
      return {
        status: 200,
        body: {
          success: true,
          data: {
            agent_id: AGENT_ID,
            agent_version: AGENT_VERSION,
            decision: 'cache_bypass',
            directives: [{ action: 'bypass', reason: 'sensitive_data' }],
            confidence: 1.0,
            constraints_evaluated: constraintsEvaluated,
            classification: CLASSIFICATION,
          },
        },
      };
    }

    if (!meta.success) {
      return {
        status: 200,
        body: {
          success: true,
          data: {
            agent_id: AGENT_ID,
            agent_version: AGENT_VERSION,
            decision: 'cache_bypass',
            directives: [{ action: 'bypass', reason: 'unsuccessful_response' }],
            confidence: 1.0,
            constraints_evaluated: constraintsEvaluated,
            classification: CLASSIFICATION,
          },
        },
      };
    }

    constraintsEvaluated.push({
      constraint_type: 'cache',
      name: 'write_eligibility',
      passed: true,
      details: `Response size: ${meta.response_size_bytes} bytes, latency: ${meta.latency_ms}ms`,
    });

    return {
      status: 200,
      body: {
        success: true,
        data: {
          agent_id: AGENT_ID,
          agent_version: AGENT_VERSION,
          decision: 'cache_write',
          directives: [
            {
              action: 'write',
              layers: ['L1'],
              cache_key: `cache:${input.content_hash}`,
              ttl_seconds: 3600,
            },
          ],
          confidence: 0.9,
          constraints_evaluated: constraintsEvaluated,
          classification: CLASSIFICATION,
        },
      },
    };
  }

  // Cache miss — nothing cached, not a write
  return {
    status: 200,
    body: {
      success: true,
      data: {
        agent_id: AGENT_ID,
        agent_version: AGENT_VERSION,
        decision: 'cache_miss',
        directives: [
          { action: 'proceed', reason: 'no_cache_entry' },
          { action: 'write_after_response', layers: ['L1'], ttl_seconds: 3600 },
        ],
        confidence: 0.9,
        constraints_evaluated: constraintsEvaluated,
        classification: CLASSIFICATION,
      },
    },
  };
}

function inspect() {
  return {
    status: 200,
    body: {
      success: true,
      data: {
        agent_id: AGENT_ID,
        agent_version: AGENT_VERSION,
        classification: CLASSIFICATION,
        decision_type: 'cache_strategy_decision',
        decisions: ['cache_hit', 'cache_miss', 'cache_write', 'cache_bypass', 'cache_invalidate'],
        contract: cachingContract,
        capabilities: [
          'Evaluate cache eligibility',
          'Apply cache read/write/bypass decisions',
          'Emit cache directives',
          'Semantic caching support',
          'Multi-layer cache recommendations',
          'Stale-while-revalidate support',
        ],
        endpoints: [
          { method: 'POST', path: '/v1/edge/caching', description: 'Evaluate caching strategy' },
          { method: 'GET',  path: '/v1/edge/caching', description: 'Inspect agent configuration' },
        ],
        non_responsibilities: [
          'Perform orchestration (LLM-Orchestrator)',
          'Modify policies dynamically',
          'Trigger retries (Orchestrator responsibility)',
          'Emit alerts (Sentinel)',
          'Perform analytics (Observatory/Latency-Lens)',
          'Persist state locally (use ruvector-service)',
          'Store actual response payloads (only metadata)',
        ],
      },
    },
  };
}

module.exports = handle;
