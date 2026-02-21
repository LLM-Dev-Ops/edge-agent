'use strict';

const { circuitBreakerContract, validateRequest } = require('../contracts');

const AGENT_ID = 'circuit-breaker-agent';
const AGENT_VERSION = '0.1.0';
const CLASSIFICATION = 'PROTECTION / GUARD';

const DEFAULT_CONFIG = {
  failure_threshold: 5,
  success_threshold: 3,
  timeout_seconds: 60,
  half_open_max_requests: 3,
};

/**
 * Circuit Breaker Agent handler.
 *
 * Evaluates circuit breaker state for a given provider and returns
 * allow/block decisions based on failure/success counts and state.
 *
 * POST /v1/edge/circuit-breaker
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
  const validation = validateRequest(body, circuitBreakerContract.request);
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

  const config = { ...DEFAULT_CONFIG, ...(body.config || {}) };
  const state = (body.current_state || 'closed').toLowerCase();
  const failureCount = body.failure_count || 0;
  const successCount = body.success_count || 0;

  let decision, newState, newFailureCount, newSuccessCount, stateTransition;

  switch (state) {
    case 'open':
      decision = { action: 'block', reason: 'Circuit is OPEN — requests blocked' };
      newState = 'open';
      newFailureCount = failureCount;
      newSuccessCount = 0;
      break;

    case 'half_open':
      if (body.success_signal) {
        newSuccessCount = successCount + 1;
        newFailureCount = failureCount;
        if (newSuccessCount >= config.success_threshold) {
          decision = { action: 'allow', reason: 'Success threshold reached in HALF_OPEN — closing circuit' };
          newState = 'closed';
          stateTransition = { from: 'half_open', to: 'closed', reason: 'success_threshold_reached' };
        } else {
          decision = { action: 'allow', reason: 'HALF_OPEN — allowing probe request' };
          newState = 'half_open';
        }
      } else if (body.failure_signal) {
        decision = { action: 'block', reason: 'Failure in HALF_OPEN — reopening circuit' };
        newState = 'open';
        newFailureCount = failureCount + 1;
        newSuccessCount = 0;
        stateTransition = { from: 'half_open', to: 'open', reason: 'failure_in_half_open' };
      } else {
        decision = { action: 'allow', reason: 'HALF_OPEN — allowing probe request' };
        newState = 'half_open';
        newFailureCount = failureCount;
        newSuccessCount = successCount;
      }
      break;

    case 'closed':
    default:
      if (body.failure_signal) {
        newFailureCount = failureCount + 1;
        newSuccessCount = 0;
        if (newFailureCount >= config.failure_threshold) {
          decision = { action: 'block', reason: 'Failure threshold reached — opening circuit' };
          newState = 'open';
          stateTransition = { from: 'closed', to: 'open', reason: 'failure_threshold_reached' };
        } else {
          decision = { action: 'allow', reason: 'CLOSED — failure recorded but threshold not reached' };
          newState = 'closed';
        }
      } else {
        decision = { action: 'allow', reason: 'Circuit CLOSED — requests allowed' };
        newState = 'closed';
        newFailureCount = failureCount;
        newSuccessCount = body.success_signal ? successCount + 1 : successCount;
      }
      break;
  }

  return {
    status: 200,
    body: {
      success: true,
      data: {
        agent_id: AGENT_ID,
        agent_version: AGENT_VERSION,
        provider_id: body.provider_id,
        decision,
        new_state: newState,
        new_failure_count: newFailureCount,
        new_success_count: newSuccessCount,
        state_transition: stateTransition || null,
        config,
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
        decision_type: 'circuit_breaker_decision',
        default_config: DEFAULT_CONFIG,
        contract: circuitBreakerContract,
        states: ['closed', 'open', 'half_open'],
        endpoints: [
          { method: 'POST', path: '/v1/edge/circuit-breaker', description: 'Evaluate circuit breaker state' },
          { method: 'GET',  path: '/v1/edge/circuit-breaker', description: 'Inspect agent configuration' },
        ],
        non_responsibilities: [
          'Perform orchestration (LLM-Orchestrator)',
          'Modify policies dynamically',
          'Trigger retries (Orchestrator responsibility)',
          'Emit alerts (Sentinel)',
          'Perform analytics (Observatory/Latency-Lens)',
          'Persist state locally (use ruvector-service)',
        ],
      },
    },
  };
}

module.exports = handle;
