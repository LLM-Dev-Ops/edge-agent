'use strict';

const ALLOWED_METHODS = 'GET, POST, OPTIONS';
const ALLOWED_HEADERS = 'Content-Type, Authorization, X-Correlation-Id, X-Parent-Span-Id, X-Execution-Id';
const MAX_AGE = '3600';

function allowedOrigins() {
  const configured = process.env.CORS_ALLOWED_ORIGINS;
  if (!configured) {
    return ['*'];
  }

  return configured
    .split(',')
    .map((origin) => origin.trim())
    .filter(Boolean);
}

function requestOrigin(req) {
  if (typeof req.get === 'function') {
    return req.get('origin');
  }

  return req.headers && (req.headers.origin || req.headers.Origin);
}

function setCorsHeaders(req, res) {
  const origins = allowedOrigins();
  const origin = requestOrigin(req);
  const wildcard = origins.includes('*');
  const permitted = wildcard || (origin && origins.includes(origin));

  if (wildcard) {
    res.set('Access-Control-Allow-Origin', '*');
  } else {
    if (typeof res.vary === 'function') res.vary('Origin');
    else res.set('Vary', 'Origin');

    if (permitted) {
      res.set('Access-Control-Allow-Origin', origin);
    }
  }

  res.set('Access-Control-Allow-Methods', ALLOWED_METHODS);
  res.set('Access-Control-Allow-Headers', ALLOWED_HEADERS);
  res.set('Access-Control-Max-Age', MAX_AGE);

  return Boolean(permitted);
}

function handlePreflight(req, res) {
  if (!setCorsHeaders(req, res)) {
    return res.status(403).json({
      error: { code: 'ORIGIN_NOT_ALLOWED', message: 'Origin is not allowed' },
    });
  }

  res.status(204).send('');
}

module.exports = { setCorsHeaders, handlePreflight };
