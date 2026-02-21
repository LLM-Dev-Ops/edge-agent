'use strict';

const ALLOWED_ORIGINS = process.env.CORS_ALLOWED_ORIGINS
  ? process.env.CORS_ALLOWED_ORIGINS.split(',')
  : ['*'];

const ALLOWED_METHODS = 'GET, POST, OPTIONS';
const ALLOWED_HEADERS = 'Content-Type, Authorization, X-Correlation-Id, X-Parent-Span-Id, X-Execution-Id';
const MAX_AGE = '3600';

function setCorsHeaders(res) {
  res.set('Access-Control-Allow-Origin', ALLOWED_ORIGINS.join(', '));
  res.set('Access-Control-Allow-Methods', ALLOWED_METHODS);
  res.set('Access-Control-Allow-Headers', ALLOWED_HEADERS);
  res.set('Access-Control-Max-Age', MAX_AGE);
}

function handlePreflight(_req, res) {
  setCorsHeaders(res);
  res.status(204).send('');
}

module.exports = { setCorsHeaders, handlePreflight };
