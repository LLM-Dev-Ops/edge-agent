'use strict';

const { toolInvokeContract, validateRequest } = require('../contracts');

const AGENT_ID = 'tool-invocation-agent';
const AGENT_VERSION = '0.1.0';
const CLASSIFICATION = 'EXECUTION CONTROL';

/**
 * Tool Invocation Agent handler.
 *
 * Validates tool invocation requests against schema constraints
 * and proxies to the upstream Rust service when available.
 *
 * POST /v1/edge/tool-invoke
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
  const validation = validateRequest(body, toolInvokeContract.request);
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

  // Validate tool name is present and non-empty
  if (!body.tool || !body.tool.name || body.tool.name.trim() === '') {
    return {
      status: 400,
      body: {
        success: false,
        error: { code: 'SCHEMA_VALIDATION_ERROR', message: 'tool.name is required and must be non-empty' },
      },
      layerStatus: 'completed',
    };
  }

  // Check blocked tools constraint
  if (body.constraints && body.constraints.blocked_tools) {
    if (body.constraints.blocked_tools.includes(body.tool.name)) {
      return {
        status: 403,
        body: {
          success: false,
          error: {
            code: 'TOOL_BLOCKED',
            message: `Tool "${body.tool.name}" is blocked by policy`,
          },
          data: {
            agent_id: AGENT_ID,
            decision: 'block',
            tool_name: body.tool.name,
            reason: 'Tool is in blocked_tools list',
          },
        },
        layerStatus: 'completed',
      };
    }
  }

  // Check allowed tools constraint
  if (body.constraints && body.constraints.allowed_tools && body.constraints.allowed_tools.length > 0) {
    if (!body.constraints.allowed_tools.includes(body.tool.name)) {
      return {
        status: 403,
        body: {
          success: false,
          error: {
            code: 'TOOL_NOT_ALLOWED',
            message: `Tool "${body.tool.name}" is not in allowed_tools list`,
          },
          data: {
            agent_id: AGENT_ID,
            decision: 'block',
            tool_name: body.tool.name,
            reason: 'Tool is not in allowed_tools list',
          },
        },
        layerStatus: 'completed',
      };
    }
  }

  return {
    status: 200,
    body: {
      success: true,
      data: {
        agent_id: AGENT_ID,
        agent_version: AGENT_VERSION,
        decision: 'allow',
        tool_name: body.tool.name,
        constraints_evaluated: body.constraints ? Object.keys(body.constraints).length : 0,
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
        decision_type: 'tool_invocation_decision',
        contract: toolInvokeContract,
        endpoints: [
          { method: 'POST', path: '/v1/edge/tool-invoke', description: 'Process a tool invocation request' },
          { method: 'GET',  path: '/v1/edge/tool-invoke', description: 'Inspect agent configuration' },
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
