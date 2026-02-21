'use strict';

const { executionGuardContract, validateRequest } = require('../contracts');

const AGENT_ID = 'execution-guard-agent';
const AGENT_VERSION = '0.1.0';
const CLASSIFICATION = 'PROTECTION / GUARD';

const DEFAULT_LIMITS = {
  max_input_tokens: 100000,
  max_output_tokens: 4096,
  max_total_tokens: 104096,
  max_cost_micros: 500000,
  max_concurrency: 50,
  max_rate_per_minute: 1000,
  allowed_models: [],
  blocked_models: [],
};

/**
 * Execution Guard Agent handler.
 *
 * Enforces execution safety constraints: token limits, cost limits,
 * rate limits, concurrency limits, and model restrictions.
 *
 * POST /v1/edge/execution-guard
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
  const validation = validateRequest(body, executionGuardContract.request);
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

  const envelope = body.envelope;
  const limits = { ...DEFAULT_LIMITS, ...(body.limits || {}) };
  const constraintsEvaluated = [];
  const violations = [];

  // Token limit checks
  if (envelope.input_tokens > limits.max_input_tokens) {
    violations.push({
      code: 'TOKEN_LIMIT_EXCEEDED',
      message: `Input tokens ${envelope.input_tokens} exceeds limit ${limits.max_input_tokens}`,
    });
  }
  constraintsEvaluated.push({
    constraint_type: 'resource',
    name: 'input_token_limit',
    passed: envelope.input_tokens <= limits.max_input_tokens,
    details: `${envelope.input_tokens} / ${limits.max_input_tokens}`,
  });

  if (envelope.max_output_tokens > limits.max_output_tokens) {
    violations.push({
      code: 'TOKEN_LIMIT_EXCEEDED',
      message: `Max output tokens ${envelope.max_output_tokens} exceeds limit ${limits.max_output_tokens}`,
    });
  }
  constraintsEvaluated.push({
    constraint_type: 'resource',
    name: 'output_token_limit',
    passed: envelope.max_output_tokens <= limits.max_output_tokens,
    details: `${envelope.max_output_tokens} / ${limits.max_output_tokens}`,
  });

  const totalTokens = envelope.input_tokens + envelope.max_output_tokens;
  constraintsEvaluated.push({
    constraint_type: 'resource',
    name: 'total_token_limit',
    passed: totalTokens <= limits.max_total_tokens,
    details: `${totalTokens} / ${limits.max_total_tokens}`,
  });
  if (totalTokens > limits.max_total_tokens) {
    violations.push({
      code: 'TOKEN_LIMIT_EXCEEDED',
      message: `Total tokens ${totalTokens} exceeds limit ${limits.max_total_tokens}`,
    });
  }

  // Cost limit check
  if (envelope.estimated_cost_micros != null && envelope.estimated_cost_micros > limits.max_cost_micros) {
    violations.push({
      code: 'COST_LIMIT_EXCEEDED',
      message: `Estimated cost ${envelope.estimated_cost_micros} micros exceeds limit ${limits.max_cost_micros}`,
    });
  }
  constraintsEvaluated.push({
    constraint_type: 'resource',
    name: 'cost_limit',
    passed: envelope.estimated_cost_micros == null || envelope.estimated_cost_micros <= limits.max_cost_micros,
    details: envelope.estimated_cost_micros != null
      ? `${envelope.estimated_cost_micros} / ${limits.max_cost_micros}`
      : 'not estimated',
  });

  // Concurrency limit check
  constraintsEvaluated.push({
    constraint_type: 'resource',
    name: 'concurrency_limit',
    passed: envelope.current_concurrency <= limits.max_concurrency,
    details: `${envelope.current_concurrency} / ${limits.max_concurrency}`,
  });
  if (envelope.current_concurrency > limits.max_concurrency) {
    violations.push({
      code: 'CONCURRENCY_LIMIT_EXCEEDED',
      message: `Concurrency ${envelope.current_concurrency} exceeds limit ${limits.max_concurrency}`,
    });
  }

  // Rate limit check
  if (body.current_rate != null && body.current_rate > limits.max_rate_per_minute) {
    violations.push({
      code: 'RATE_LIMIT_EXCEEDED',
      message: `Rate ${body.current_rate}/min exceeds limit ${limits.max_rate_per_minute}/min`,
    });
  }
  constraintsEvaluated.push({
    constraint_type: 'rate_limit',
    name: 'rate_limit',
    passed: body.current_rate == null || body.current_rate <= limits.max_rate_per_minute,
    details: body.current_rate != null
      ? `${body.current_rate} / ${limits.max_rate_per_minute} per minute`
      : 'not provided',
  });

  // Model restriction check
  if (limits.blocked_models.length > 0 && limits.blocked_models.includes(envelope.model)) {
    violations.push({
      code: 'MODEL_BLOCKED',
      message: `Model "${envelope.model}" is blocked by policy`,
    });
  }
  if (limits.allowed_models.length > 0 && !limits.allowed_models.includes(envelope.model)) {
    violations.push({
      code: 'MODEL_BLOCKED',
      message: `Model "${envelope.model}" is not in allowed models list`,
    });
  }
  constraintsEvaluated.push({
    constraint_type: 'policy',
    name: 'model_restriction',
    passed: violations.filter(v => v.code === 'MODEL_BLOCKED').length === 0,
    details: `model: ${envelope.model}`,
  });

  const allowed = violations.length === 0;
  const status = allowed ? 200 : 403;

  return {
    status,
    body: {
      success: allowed,
      data: {
        agent_id: AGENT_ID,
        agent_version: AGENT_VERSION,
        decision: allowed ? 'allow' : 'block',
        violations: violations.length > 0 ? violations : undefined,
        constraints_evaluated: constraintsEvaluated,
        envelope_summary: {
          request_id: envelope.request_id,
          model: envelope.model,
          input_tokens: envelope.input_tokens,
          max_output_tokens: envelope.max_output_tokens,
          current_concurrency: envelope.current_concurrency,
        },
        classification: CLASSIFICATION,
      },
      error: !allowed
        ? { code: violations[0].code, message: violations[0].message }
        : undefined,
    },
    layerStatus: 'completed',
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
        decision_type: 'execution_guard_decision',
        default_limits: DEFAULT_LIMITS,
        contract: executionGuardContract,
        constraints_enforced: [
          'Token limits (input, output, total)',
          'Cost limits (per-request budget)',
          'Rate limits (requests per window)',
          'Concurrency limits (max parallel executions)',
          'Model restrictions (allowed/blocked models)',
        ],
        endpoints: [
          { method: 'POST', path: '/v1/edge/execution-guard', description: 'Evaluate execution guard constraints' },
          { method: 'GET',  path: '/v1/edge/execution-guard', description: 'Inspect agent configuration' },
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
