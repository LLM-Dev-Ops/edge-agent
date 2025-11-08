/**
 * LLM Edge Agent - Stress Test
 *
 * Tests system limits by gradually increasing load until breaking point
 * Identifies maximum capacity and degradation patterns
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const requestDuration = new Trend('request_duration');
const activeConnections = new Gauge('active_connections');
const requestCount = new Counter('request_count');

export const options = {
  stages: [
    { duration: '2m', target: 100 },   // Normal load
    { duration: '5m', target: 200 },   // Increase to 200
    { duration: '5m', target: 400 },   // Increase to 400
    { duration: '5m', target: 600 },   // Increase to 600
    { duration: '5m', target: 800 },   // Increase to 800
    { duration: '5m', target: 1000 },  // Maximum load
    { duration: '2m', target: 0 },     // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<10000'], // Allow high latency at stress levels
    'http_req_failed': ['rate<0.1'],      // Allow up to 10% errors
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export default function () {
  const payload = JSON.stringify({
    model: 'gpt-3.5-turbo',
    messages: [
      {
        role: 'user',
        content: 'Stress test message',
      },
    ],
    max_tokens: 50,
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${__ENV.API_KEY || 'test-key'}`,
    },
  };

  const response = http.post(`${BASE_URL}/v1/chat/completions`, payload, params);

  requestCount.add(1);
  requestDuration.add(response.timings.duration);

  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'has response': (r) => r.body && r.body.length > 0,
  });

  if (!success) {
    errorRate.add(1);
    console.log(`Error at VU ${__VU}: ${response.status} - ${response.body.substring(0, 100)}`);
  } else {
    errorRate.add(0);
  }

  // Very short sleep to maximize load
  sleep(0.05);
}

export function handleSummary(data) {
  console.log('\n\n=== Stress Test Results ===\n');
  console.log(`Total Requests: ${data.metrics.request_count.values.count}`);
  console.log(`Failed Requests: ${data.metrics.http_req_failed.values.passes}`);
  console.log(`Error Rate: ${(data.metrics.errors.values.rate * 100).toFixed(2)}%`);
  console.log(`\nLatency:`);
  console.log(`  P50: ${data.metrics.http_req_duration.values['p(50)'].toFixed(2)}ms`);
  console.log(`  P95: ${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms`);
  console.log(`  P99: ${data.metrics.http_req_duration.values['p(99)'].toFixed(2)}ms`);
  console.log(`  Max: ${data.metrics.http_req_duration.values.max.toFixed(2)}ms`);

  return {
    'stress-test-results.json': JSON.stringify(data, null, 2),
  };
}
