'use strict';

/**
 * Contract schemas for all 5 edge agents.
 *
 * These mirror the Rust contracts in crates/agentics-contracts/
 * and crates/llm-edge-agents/src/contracts.rs.
 */

const toolInvokeContract = {
  request: {
    type: 'object',
    required: ['tool', 'arguments'],
    properties: {
      tool: {
        type: 'object',
        required: ['name'],
        properties: {
          name: { type: 'string' },
          description: { type: 'string' },
          parameters: { type: 'array', items: { type: 'object' } },
          capabilities: { type: 'array', items: { type: 'string' } },
        },
      },
      arguments: { type: 'object' },
      context: { $ref: '#/definitions/executionContext' },
      constraints: {
        type: 'object',
        properties: {
          max_execution_time_ms: { type: 'integer' },
          max_memory_bytes: { type: 'integer' },
          allowed_tools: { type: 'array', items: { type: 'string' } },
          blocked_tools: { type: 'array', items: { type: 'string' } },
          max_concurrent: { type: 'integer' },
          required_capabilities: { type: 'array', items: { type: 'string' } },
        },
      },
    },
  },
  response: {
    type: 'object',
    properties: {
      success: { type: 'boolean' },
      data: { type: 'object' },
      error: { $ref: '#/definitions/apiError' },
    },
  },
};

const circuitBreakerContract = {
  request: {
    type: 'object',
    required: ['provider_id', 'execution_ref'],
    properties: {
      provider_id: { type: 'string' },
      execution_ref: { type: 'string' },
      action: { type: 'string', enum: ['check', 'record_failure', 'record_success'] },
      current_state: { type: 'string', enum: ['closed', 'open', 'half_open'] },
      failure_count: { type: 'integer', minimum: 0 },
      success_count: { type: 'integer', minimum: 0 },
      failure_signal: {
        type: 'object',
        properties: {
          provider_id: { type: 'string' },
          failure_type: { type: 'string' },
        },
      },
      success_signal: {
        type: 'object',
        properties: {
          provider_id: { type: 'string' },
        },
      },
      config: { $ref: '#/definitions/circuitBreakerConfig' },
    },
  },
  response: {
    type: 'object',
    properties: {
      success: { type: 'boolean' },
      data: {
        type: 'object',
        properties: {
          decision: { type: 'object' },
          new_state: { type: 'string' },
          new_failure_count: { type: 'integer' },
          new_success_count: { type: 'integer' },
          state_transition: { type: 'object' },
        },
      },
      error: { type: 'string' },
    },
  },
};

const failoverContract = {
  request: {
    type: 'object',
    required: ['execution_context', 'primary_target'],
    properties: {
      execution_context: { $ref: '#/definitions/executionContext' },
      primary_target: { $ref: '#/definitions/targetProvider' },
      alternate_targets: {
        type: 'array',
        items: { $ref: '#/definitions/targetProvider' },
      },
      policy: { $ref: '#/definitions/failoverPolicy' },
    },
  },
  response: {
    type: 'object',
    properties: {
      success: { type: 'boolean' },
      data: {
        type: 'object',
        properties: {
          decision: { type: 'object' },
          constraints_applied: { type: 'array' },
          duration_us: { type: 'integer' },
        },
      },
      error: { type: 'string' },
    },
  },
};

const executionGuardContract = {
  request: {
    type: 'object',
    required: ['envelope'],
    properties: {
      envelope: {
        type: 'object',
        required: ['request_id', 'model', 'input_tokens', 'max_output_tokens'],
        properties: {
          request_id: { type: 'string' },
          model: { type: 'string' },
          provider: { type: 'string' },
          input_tokens: { type: 'integer', minimum: 0 },
          max_output_tokens: { type: 'integer', minimum: 0 },
          estimated_duration_ms: { type: 'integer', minimum: 0 },
          estimated_memory_bytes: { type: 'integer', minimum: 0 },
          estimated_cost_micros: { type: 'integer', minimum: 0 },
          current_concurrency: { type: 'integer', minimum: 0 },
          metadata: { type: 'object' },
        },
      },
      context: { $ref: '#/definitions/executionContext' },
      limits: {
        type: 'object',
        properties: {
          max_input_tokens: { type: 'integer', minimum: 0 },
          max_output_tokens: { type: 'integer', minimum: 0 },
          max_total_tokens: { type: 'integer', minimum: 0 },
          max_cost_micros: { type: 'integer', minimum: 0 },
          max_concurrency: { type: 'integer', minimum: 0 },
          max_rate_per_minute: { type: 'integer', minimum: 0 },
          allowed_models: { type: 'array', items: { type: 'string' } },
          blocked_models: { type: 'array', items: { type: 'string' } },
        },
      },
      current_rate: { type: 'integer', minimum: 0 },
    },
  },
  response: {
    type: 'object',
    properties: {
      success: { type: 'boolean' },
      data: { type: 'object' },
      error: { $ref: '#/definitions/apiError' },
    },
  },
};

const cachingContract = {
  request: {
    type: 'object',
    required: ['input'],
    properties: {
      input: {
        type: 'object',
        required: ['execution_ref', 'request_type', 'content_hash', 'prompt_hash'],
        properties: {
          execution_ref: { type: 'string' },
          request_type: { type: 'string' },
          provider_id: { type: 'string' },
          model_id: { type: 'string' },
          content_hash: { type: 'string' },
          prompt_hash: { type: 'string' },
          semantic_hash: { type: 'string' },
          cache_relevant_params: {
            type: 'object',
            properties: {
              temperature: { type: 'number' },
              top_p: { type: 'number' },
              max_tokens: { type: 'integer' },
              seed: { type: 'integer' },
              response_format: { type: 'string' },
              stream: { type: 'boolean' },
              system_prompt_hash: { type: 'string' },
            },
          },
          policy: { type: 'object' },
          existing_entry: { type: 'object' },
          check_cache: { type: 'boolean' },
          is_write_operation: { type: 'boolean' },
          response_metadata: { type: 'object' },
          tenant_id: { type: 'string' },
          user_id_hash: { type: 'string' },
        },
      },
      context: { $ref: '#/definitions/executionContext' },
      config_override: { type: 'object' },
    },
  },
  response: {
    type: 'object',
    properties: {
      decision: { type: 'string', enum: ['cache_hit', 'cache_miss', 'cache_write', 'cache_bypass', 'cache_invalidate'] },
      directives: { type: 'array' },
      confidence: { type: 'number', minimum: 0, maximum: 1 },
    },
  },
};

const definitions = {
  executionContext: {
    type: 'object',
    properties: {
      execution_ref: { type: 'string' },
      execution_id: { type: 'string' },
      parent_span_id: { type: 'string' },
      request_id: { type: 'string' },
      tenant_id: { type: 'string' },
      user_id: { type: 'string' },
      session_id: { type: 'string' },
      source: { type: 'string' },
      tags: { type: 'object' },
    },
  },
  apiError: {
    type: 'object',
    properties: {
      code: { type: 'string' },
      message: { type: 'string' },
      details: { type: 'object' },
    },
  },
  targetProvider: {
    type: 'object',
    required: ['provider_id', 'provider_name', 'model'],
    properties: {
      provider_id: { type: 'string' },
      provider_name: { type: 'string' },
      model: { type: 'string' },
      endpoint: { type: 'string' },
      priority: { type: 'integer' },
      circuit_state: { type: 'string', enum: ['closed', 'open', 'half_open'] },
      health: { type: 'string', enum: ['healthy', 'degraded', 'unhealthy'] },
      metadata: { type: 'object' },
    },
  },
  failoverPolicy: {
    type: 'object',
    properties: {
      max_retries: { type: 'integer' },
      failover_strategy: { type: 'string' },
      timeout_ms: { type: 'integer' },
    },
  },
  circuitBreakerConfig: {
    type: 'object',
    properties: {
      failure_threshold: { type: 'integer' },
      success_threshold: { type: 'integer' },
      timeout_seconds: { type: 'integer' },
      half_open_max_requests: { type: 'integer' },
    },
  },
};

/**
 * Enforce the schema subset used by the published agent contracts: local
 * definition refs, object/array/scalar types, required fields, enum and minimum.
 * This is not a general-purpose JSON Schema implementation. Unknown request
 * properties remain permitted for forward compatibility; no coercion is used.
 * Returns { valid: true } or { valid: false, errors: [...] }.
 */
function validateRequest(body, schema) {
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    return { valid: false, errors: ['Request body must be a JSON object'] };
  }

  const errors = [];
  let visited = 0;
  function check(value, rule, path, depth) {
    if (errors.length >= 32) return;
    if (++visited > 10000 || depth > 32) {
      errors.push('Request exceeds contract validation budget');
      return;
    }
    if (rule.$ref) {
      const prefix = '#/definitions/';
      const name = rule.$ref.startsWith(prefix) ? rule.$ref.slice(prefix.length) : '';
      if (!Object.hasOwn(definitions, name)) {
        errors.push(`${path}: unsupported contract reference`);
        return;
      }
      check(value, definitions[name], path, depth + 1);
      return;
    }
    const matches = {
      object: () => value !== null && typeof value === 'object' && !Array.isArray(value),
      array: () => Array.isArray(value),
      string: () => typeof value === 'string',
      boolean: () => typeof value === 'boolean',
      number: () => typeof value === 'number' && Number.isFinite(value),
      integer: () => Number.isSafeInteger(value),
    };
    if (rule.type && (!Object.hasOwn(matches, rule.type) || !matches[rule.type]())) {
      errors.push(`${path}: expected ${rule.type}`);
      return;
    }
    if (rule.enum && !rule.enum.includes(value)) errors.push(`${path}: value is not allowed`);
    if (rule.minimum !== undefined && value < rule.minimum) errors.push(`${path}: below minimum ${rule.minimum}`);
    if (rule.type === 'object') {
      for (const field of rule.required || []) {
        if (!Object.hasOwn(value, field) || value[field] == null) {
          errors.push(`Missing required field: ${path}.${field}`);
          if (errors.length >= 32) return;
        }
      }
      for (const [field, property] of Object.entries(rule.properties || {})) {
        if (Object.hasOwn(value, field) && value[field] !== undefined) check(value[field], property, `${path}.${field}`, depth + 1);
        if (errors.length >= 32) return;
      }
    } else if (rule.type === 'array' && rule.items) {
      for (let i = 0; i < value.length; i++) {
        check(value[i], rule.items, `${path}[${i}]`, depth + 1);
        if (errors.length >= 32 || visited > 10000) return;
      }
    }
  }
  check(body, schema, 'body', 0);

  return errors.length > 0 ? { valid: false, errors } : { valid: true, errors: [] };
}

module.exports = {
  toolInvokeContract,
  circuitBreakerContract,
  failoverContract,
  executionGuardContract,
  cachingContract,
  definitions,
  validateRequest,
};
