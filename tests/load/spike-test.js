/**
 * LLM Edge Agent - Spike Test
 *
 * Tests system behavior under sudden traffic spikes
 * Simulates sudden increase to 500 VUs then back to normal
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const circuitBreakerOpen = new Rate('circuit_breaker_open');
const requestDuration = new Trend('request_duration');

export const options = {
  stages: [
    { duration: '1m', target: 100 },   // Normal load
    { duration: '30s', target: 500 },  // Spike to 500 VUs
    { duration: '2m', target: 500 },   // Hold spike
    { duration: '1m', target: 100 },   // Back to normal
    { duration: '2m', target: 100 },   // Recovery period
    { duration: '30s', target: 0 },    // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<5000'],  // Allow higher latency during spike
    'http_req_failed': ['rate<0.05'],     // Allow up to 5% errors during spike
    'errors': ['rate<0.05'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

const testPrompts = [
  'Quick test prompt 1',
  'Quick test prompt 2',
  'Quick test prompt 3',
];

export default function () {
  const prompt = testPrompts[Math.floor(Math.random() * testPrompts.length)];

  const payload = JSON.stringify({
    model: 'gpt-3.5-turbo',
    messages: [{ role: 'user', content: prompt }],
    max_tokens: 50,
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${__ENV.API_KEY || 'test-key'}`,
    },
  };

  const response = http.post(`${BASE_URL}/v1/chat/completions`, payload, params);

  // Check for circuit breaker
  if (response.status === 503) {
    circuitBreakerOpen.add(1);
  } else {
    circuitBreakerOpen.add(0);
  }

  const success = check(response, {
    'status is 200 or 503': (r) => r.status === 200 || r.status === 503,
    'response time acceptable': (r) => r.timings.duration < 10000,
  });

  if (!success) {
    errorRate.add(1);
  } else {
    errorRate.add(0);
  }

  requestDuration.add(response.timings.duration);

  // Minimal sleep during spike test
  sleep(0.1);
}
