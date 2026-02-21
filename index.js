'use strict';

const crypto = require('crypto');
const { setCorsHeaders, handlePreflight } = require('./cloud-functions/middleware');
const { route } = require('./cloud-functions/router');

/**
 * Cloud Function entry point for edge-agents.
 *
 * Deploy:
 *   gcloud functions deploy edge-agents --runtime nodejs20 --trigger-http \
 *     --region us-central1 --project agentics-dev --entry-point handler \
 *     --memory 512MB --timeout 30s --no-allow-unauthenticated
 *
 * @param {import('@google-cloud/functions-framework').Request} req
 * @param {import('@google-cloud/functions-framework').Response} res
 */
exports.handler = async (req, res) => {
  // CORS preflight
  if (req.method === 'OPTIONS') {
    return handlePreflight(req, res);
  }

  setCorsHeaders(res);

  const start = Date.now();
  const trace_id = req.headers['x-correlation-id'] || crypto.randomUUID();
  const execution_id = crypto.randomUUID();

  try {
    const result = await route(req);
    const elapsed = Date.now() - start;

    const body = {
      ...result.body,
      execution_metadata: {
        trace_id,
        timestamp: new Date().toISOString(),
        service: 'edge-agents',
        execution_id,
      },
      layers_executed: [
        { layer: 'AGENT_ROUTING', status: 'completed' },
        { layer: result.layer, status: result.layerStatus || 'completed', duration_ms: elapsed },
      ],
    };

    res.status(result.status || 200).json(body);
  } catch (err) {
    const elapsed = Date.now() - start;

    res.status(500).json({
      error: { code: 'INTERNAL_ERROR', message: err.message },
      execution_metadata: {
        trace_id,
        timestamp: new Date().toISOString(),
        service: 'edge-agents',
        execution_id,
      },
      layers_executed: [
        { layer: 'AGENT_ROUTING', status: 'completed' },
        { layer: 'EDGE_ERROR', status: 'failed', duration_ms: elapsed },
      ],
    });
  }
};
