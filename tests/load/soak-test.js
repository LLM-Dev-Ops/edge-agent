/**
 * LLM Edge Agent - Soak Test (Endurance Test)
 *
 * Tests system stability over extended period
 * Runs for 4 hours at moderate load to detect memory leaks and degradation
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const memoryLeakIndicator = new Trend('response_time_trend');
const requestCount = new Counter('request_count');

export const options = {
  stages: [
    { duration: '5m', target: 50 },     // Ramp up
    { duration: '4h', target: 50 },     // Sustained load for 4 hours
    { duration: '5m', target: 0 },      // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<2000'], // Latency should remain stable
    'http_req_failed': ['rate<0.01'],
    'errors': ['rate<0.01'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

const prompts = [
  'What is machine learning?',
  'Explain cloud computing',
  'How does AI work?',
  'What is DevOps?',
  'Describe microservices architecture',
];

export default function () {
  const prompt = prompts[Math.floor(Math.random() * prompts.length)];

  const payload = JSON.stringify({
    model: 'gpt-3.5-turbo',
    messages: [{ role: 'user', content: prompt }],
    temperature: 0.7,
    max_tokens: 100,
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${__ENV.API_KEY || 'test-key'}`,
    },
  };

  const response = http.post(`${BASE_URL}/v1/chat/completions`, payload, params);

  requestCount.add(1);
  memoryLeakIndicator.add(response.timings.duration);

  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response time stable': (r) => r.timings.duration < 2000,
    'has valid response': (r) => {
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
  } else {
    errorRate.add(0);
  }

  sleep(Math.random() * 3 + 2); // 2-5 seconds between requests
}

export function handleSummary(data) {
  // Check for performance degradation over time
  const avgDuration = data.metrics.http_req_duration.values.avg;
  const p95Duration = data.metrics.http_req_duration.values['p(95)'];

  console.log('\n\n=== Soak Test Results ===\n');
  console.log(`Duration: 4 hours`);
  console.log(`Total Requests: ${data.metrics.request_count.values.count}`);
  console.log(`Average Response Time: ${avgDuration.toFixed(2)}ms`);
  console.log(`P95 Response Time: ${p95Duration.toFixed(2)}ms`);
  console.log(`Error Rate: ${(data.metrics.errors.values.rate * 100).toFixed(2)}%`);

  // Flag potential memory leak if response time increased significantly
  if (p95Duration > 2500) {
    console.log('\n⚠️  WARNING: Response times degraded, possible memory leak');
  } else {
    console.log('\n✅ System remained stable over extended period');
  }

  return {
    'soak-test-results.json': JSON.stringify(data, null, 2),
  };
}
