'use strict';

const toolInvoke = require('./agents/tool-invoke');
const circuitBreaker = require('./agents/circuit-breaker');
const failover = require('./agents/failover');
const executionGuard = require('./agents/execution-guard');
const caching = require('./agents/caching');

const AGENTS = [
  'tool-invoke',
  'circuit-breaker',
  'failover',
  'execution-guard',
  'caching',
];

const ROUTE_TABLE = {
  '/v1/edge/tool-invoke':      { handler: toolInvoke,      layer: 'EDGE_TOOL_INVOKE' },
  '/v1/edge/circuit-breaker':  { handler: circuitBreaker,  layer: 'EDGE_CIRCUIT_BREAKER' },
  '/v1/edge/failover':         { handler: failover,        layer: 'EDGE_FAILOVER' },
  '/v1/edge/execution-guard':  { handler: executionGuard,  layer: 'EDGE_EXECUTION_GUARD' },
  '/v1/edge/caching':          { handler: caching,         layer: 'EDGE_CACHING' },
};

/**
 * Route an incoming request to the appropriate agent handler.
 * Returns { status, body, layer, layerStatus }.
 */
async function route(req) {
  const path = req.path || req.url;

  // Health endpoint
  if (path === '/health' || path === '/') {
    return health();
  }

  // Contract schema endpoint
  if (path === '/v1/edge/contracts') {
    return contracts();
  }

  const entry = ROUTE_TABLE[path];
  if (!entry) {
    return {
      status: 404,
      body: {
        error: {
          code: 'ROUTE_NOT_FOUND',
          message: `Unknown route: ${path}. Valid routes: ${Object.keys(ROUTE_TABLE).join(', ')}`,
        },
      },
      layer: 'AGENT_ROUTING',
      layerStatus: 'failed',
    };
  }

  const result = await entry.handler(req);
  return {
    status: result.status || 200,
    body: result.body,
    layer: entry.layer,
    layerStatus: result.layerStatus || 'completed',
  };
}

function health() {
  const agentStatuses = {};
  for (const name of AGENTS) {
    agentStatuses[name] = { status: 'healthy' };
  }

  return {
    status: 200,
    body: {
      status: 'healthy',
      service: 'edge-agents',
      agents: agentStatuses,
      available_routes: Object.keys(ROUTE_TABLE),
    },
    layer: 'HEALTH',
  };
}

function contracts() {
  const schemas = require('./contracts');
  return {
    status: 200,
    body: {
      service: 'edge-agents',
      agents: {
        'tool-invoke':      { route: '/v1/edge/tool-invoke',      contract: schemas.toolInvokeContract },
        'circuit-breaker':  { route: '/v1/edge/circuit-breaker',  contract: schemas.circuitBreakerContract },
        'failover':         { route: '/v1/edge/failover',         contract: schemas.failoverContract },
        'execution-guard':  { route: '/v1/edge/execution-guard',  contract: schemas.executionGuardContract },
        'caching':          { route: '/v1/edge/caching',          contract: schemas.cachingContract },
      },
      definitions: schemas.definitions,
    },
    layer: 'CONTRACTS',
  };
}

module.exports = { route, ROUTE_TABLE, AGENTS };
