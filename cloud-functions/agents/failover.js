'use strict';

const { failoverContract, validateRequest } = require('../contracts');

const AGENT_ID = 'failover-agent';
const AGENT_VERSION = '0.1.0';
const CLASSIFICATION = 'ROUTING';

/**
 * Failover Agent handler.
 *
 * Evaluates failover routing decisions based on provider health,
 * circuit state, and priority ordering.
 *
 * POST /v1/edge/failover
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
  const validation = validateRequest(body, failoverContract.request);
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

  const primary = body.primary_target;
  const alternates = body.alternate_targets || [];
  const constraintsApplied = [];

  // Evaluate primary
  const primaryHealthy = isPrimaryHealthy(primary);
  constraintsApplied.push({
    constraint_type: 'routing',
    name: 'primary_health_check',
    passed: primaryHealthy,
    details: primaryHealthy
      ? `Primary "${primary.provider_name}" is healthy`
      : `Primary "${primary.provider_name}" is unavailable (circuit: ${primary.circuit_state || 'unknown'}, health: ${primary.health || 'unknown'})`,
  });

  if (primaryHealthy) {
    return {
      status: 200,
      body: {
        success: true,
        data: {
          agent_id: AGENT_ID,
          agent_version: AGENT_VERSION,
          decision: {
            outcome: 'use_primary',
            selected_target: primary,
            reason: 'Primary target is healthy',
            confidence: 1.0,
            evaluated_targets: [primary, ...alternates],
          },
          constraints_applied: constraintsApplied,
          classification: CLASSIFICATION,
        },
      },
    };
  }

  // Evaluate alternates by priority
  const sorted = [...alternates].sort((a, b) => (a.priority || 0) - (b.priority || 0));
  for (const alt of sorted) {
    const altHealthy = isAlternateHealthy(alt);
    constraintsApplied.push({
      constraint_type: 'routing',
      name: `alternate_health_check_${alt.provider_id}`,
      passed: altHealthy,
      details: altHealthy
        ? `Alternate "${alt.provider_name}" is healthy`
        : `Alternate "${alt.provider_name}" is unavailable`,
    });

    if (altHealthy) {
      return {
        status: 200,
        body: {
          success: true,
          data: {
            agent_id: AGENT_ID,
            agent_version: AGENT_VERSION,
            decision: {
              outcome: 'failover',
              selected_target: alt,
              reason: `Primary unavailable, failing over to "${alt.provider_name}"`,
              confidence: 0.85,
              evaluated_targets: [primary, ...alternates],
            },
            constraints_applied: constraintsApplied,
            classification: CLASSIFICATION,
          },
        },
      };
    }
  }

  // All targets down
  return {
    status: 200,
    body: {
      success: true,
      data: {
        agent_id: AGENT_ID,
        agent_version: AGENT_VERSION,
        decision: {
          outcome: 'no_available_target',
          selected_target: null,
          reason: 'All targets are unavailable',
          confidence: 1.0,
          evaluated_targets: [primary, ...alternates],
        },
        constraints_applied: constraintsApplied,
        classification: CLASSIFICATION,
      },
    },
  };
}

function isPrimaryHealthy(target) {
  if (!target) return false;
  const circuit = (target.circuit_state || 'closed').toLowerCase();
  const health = (target.health || 'healthy').toLowerCase();
  return circuit !== 'open' && health !== 'unhealthy';
}

function isAlternateHealthy(target) {
  return isPrimaryHealthy(target);
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
        decision_type: 'failover_routing_decision',
        contract: failoverContract,
        outcomes: ['use_primary', 'failover', 'no_available_target'],
        endpoints: [
          { method: 'POST', path: '/v1/edge/failover', description: 'Evaluate failover routing' },
          { method: 'GET',  path: '/v1/edge/failover', description: 'Inspect agent configuration' },
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
