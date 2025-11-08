/**
 * LLM Edge Agent - Baseline Load Test
 *
 * Tests basic throughput and latency under normal load conditions
 * Target: 100 RPS sustained for 10 minutes
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const requestDuration = new Trend('request_duration');
const requestCount = new Counter('request_count');

// Test configuration
export const options = {
  stages: [
    { duration: '2m', target: 50 },   // Ramp up to 50 VUs
    { duration: '5m', target: 100 },  // Ramp up to 100 VUs
    { duration: '10m', target: 100 }, // Stay at 100 VUs
    { duration: '2m', target: 0 },    // Ramp down to 0
  ],
  thresholds: {
    'http_req_duration': ['p(95)<2000'], // 95% of requests must complete below 2s
    'http_req_duration{cached:yes}': ['p(95)<100'], // Cached requests < 100ms
    'http_req_failed': ['rate<0.01'],     // Error rate must be less than 1%
    'errors': ['rate<0.01'],
    'cache_hits': ['rate>0.5'],           // Cache hit rate should be > 50%
  },
};

// Base URL
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test data - various prompts to test caching
const prompts = [
  'What is the capital of France?',
  'Explain quantum computing in simple terms',
  'Write a haiku about coding',
  'What are the benefits of microservices?',
  'How does Redis caching work?',
  'Explain the CAP theorem',
  'What is Rust programming language?',
  'How does load balancing work?',
  'What is Docker and Kubernetes?',
  'Explain REST API design principles',
];

// Models to test
const models = [
  'gpt-3.5-turbo',
  'gpt-4',
  'claude-3-sonnet-20240229',
  'claude-3-opus-20240229',
];

export default function () {
  // Select random prompt and model
  const prompt = prompts[Math.floor(Math.random() * prompts.length)];
  const model = models[Math.floor(Math.random() * models.length)];

  const payload = JSON.stringify({
    model: model,
    messages: [
      {
        role: 'user',
        content: prompt,
      },
    ],
    temperature: 0.7,
    max_tokens: 100,
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${__ENV.API_KEY || 'test-key'}`,
    },
    tags: {
      model: model,
    },
  };

  const startTime = new Date();
  const response = http.post(`${BASE_URL}/v1/chat/completions`, payload, params);
  const duration = new Date() - startTime;

  // Record metrics
  requestCount.add(1);
  requestDuration.add(duration);

  // Check if request was cached (based on X-Cache-Status header)
  const isCached = response.headers['X-Cache-Status'] === 'HIT';
  cacheHitRate.add(isCached ? 1 : 0);

  // Validate response
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'has response body': (r) => r.body && r.body.length > 0,
    'response time OK': (r) => r.timings.duration < 2000,
    'cached response fast': (r) => !isCached || r.timings.duration < 100,
    'has choices': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.choices && body.choices.length > 0;
      } catch (e) {
        return false;
      }
    },
  });

  if (!success) {
    errorRate.add(1);
    console.error(`Request failed: ${response.status} ${response.body}`);
  } else {
    errorRate.add(0);
  }

  // Think time between requests (simulate real user behavior)
  sleep(Math.random() * 2 + 1); // 1-3 seconds
}

export function handleSummary(data) {
  return {
    'summary.json': JSON.stringify(data),
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
  };
}

function textSummary(data, options) {
  const indent = options.indent || '';
  const enableColors = options.enableColors || false;

  let summary = '\n\n';
  summary += `${indent}Baseline Load Test Summary\n`;
  summary += `${indent}${'='.repeat(50)}\n\n`;

  // Request statistics
  summary += `${indent}Total Requests: ${data.metrics.request_count.values.count}\n`;
  summary += `${indent}Failed Requests: ${data.metrics.http_req_failed.values.passes}\n`;
  summary += `${indent}Error Rate: ${(data.metrics.errors.values.rate * 100).toFixed(2)}%\n\n`;

  // Response time statistics
  summary += `${indent}Response Time:\n`;
  summary += `${indent}  avg: ${data.metrics.http_req_duration.values.avg.toFixed(2)}ms\n`;
  summary += `${indent}  min: ${data.metrics.http_req_duration.values.min.toFixed(2)}ms\n`;
  summary += `${indent}  p50: ${data.metrics.http_req_duration.values['p(50)'].toFixed(2)}ms\n`;
  summary += `${indent}  p95: ${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms\n`;
  summary += `${indent}  p99: ${data.metrics.http_req_duration.values['p(99)'].toFixed(2)}ms\n`;
  summary += `${indent}  max: ${data.metrics.http_req_duration.values.max.toFixed(2)}ms\n\n`;

  // Cache statistics
  summary += `${indent}Cache Hit Rate: ${(data.metrics.cache_hits.values.rate * 100).toFixed(2)}%\n\n`;

  // Throughput
  const duration = data.state.testRunDurationMs / 1000;
  const rps = data.metrics.request_count.values.count / duration;
  summary += `${indent}Throughput: ${rps.toFixed(2)} RPS\n`;

  return summary;
}
