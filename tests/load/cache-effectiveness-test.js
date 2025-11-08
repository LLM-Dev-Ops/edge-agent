/**
 * LLM Edge Agent - Cache Effectiveness Test
 *
 * Tests cache hit rates and cache performance under various scenarios
 * Validates L1 and L2 cache behavior
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const l1CacheHits = new Rate('l1_cache_hits');
const l2CacheHits = new Rate('l2_cache_hits');
const cacheMisses = new Rate('cache_misses');
const cachedResponseTime = new Trend('cached_response_time');
const uncachedResponseTime = new Trend('uncached_response_time');

export const options = {
  stages: [
    { duration: '1m', target: 50 },   // Warm up cache
    { duration: '5m', target: 100 },  // Test cache effectiveness
    { duration: '1m', target: 0 },    // Ramp down
  ],
  thresholds: {
    'l1_cache_hits': ['rate>0.6'],     // L1 should hit >60% of time
    'cached_response_time': ['p(95)<100'], // Cached responses < 100ms
    'uncached_response_time': ['p(95)<2000'], // Uncached < 2s
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Repeated prompts to test cache effectiveness
const repeatedPrompts = [
  'What is the capital of France?',
  'Explain quantum computing',
  'Write a haiku about nature',
  'How does caching work?',
  'What is Kubernetes?',
];

export default function () {
  // 70% chance of repeated prompt, 30% new prompt
  const isRepeated = Math.random() < 0.7;
  const prompt = isRepeated
    ? repeatedPrompts[Math.floor(Math.random() * repeatedPrompts.length)]
    : `Random prompt ${Math.random()}`;

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

  // Check cache status from response headers
  const cacheStatus = response.headers['X-Cache-Status'] || 'MISS';
  const cacheLayer = response.headers['X-Cache-Layer'] || 'NONE';

  if (cacheStatus === 'HIT') {
    if (cacheLayer === 'L1') {
      l1CacheHits.add(1);
      l2CacheHits.add(0);
      cacheMisses.add(0);
      cachedResponseTime.add(response.timings.duration);
    } else if (cacheLayer === 'L2') {
      l1CacheHits.add(0);
      l2CacheHits.add(1);
      cacheMisses.add(0);
      cachedResponseTime.add(response.timings.duration);
    }
  } else {
    l1CacheHits.add(0);
    l2CacheHits.add(0);
    cacheMisses.add(1);
    uncachedResponseTime.add(response.timings.duration);
  }

  check(response, {
    'status is 200': (r) => r.status === 200,
    'cached response fast': (r) => cacheStatus !== 'HIT' || r.timings.duration < 100,
    'L1 cache fastest': (r) => cacheLayer !== 'L1' || r.timings.duration < 50,
  });

  sleep(0.5);
}

export function handleSummary(data) {
  const totalRequests = data.metrics.iterations.values.count;
  const l1Hits = data.metrics.l1_cache_hits.values.rate * totalRequests;
  const l2Hits = data.metrics.l2_cache_hits.values.rate * totalRequests;
  const misses = data.metrics.cache_misses.values.rate * totalRequests;

  console.log('\n\n=== Cache Effectiveness Test Results ===\n');
  console.log(`Total Requests: ${totalRequests}`);
  console.log(`L1 Cache Hits: ${l1Hits.toFixed(0)} (${(data.metrics.l1_cache_hits.values.rate * 100).toFixed(2)}%)`);
  console.log(`L2 Cache Hits: ${l2Hits.toFixed(0)} (${(data.metrics.l2_cache_hits.values.rate * 100).toFixed(2)}%)`);
  console.log(`Cache Misses: ${misses.toFixed(0)} (${(data.metrics.cache_misses.values.rate * 100).toFixed(2)}%)`);
  console.log(`\nCached Response Time (P95): ${data.metrics.cached_response_time?.values['p(95)']?.toFixed(2) || 'N/A'}ms`);
  console.log(`Uncached Response Time (P95): ${data.metrics.uncached_response_time?.values['p(95)']?.toFixed(2) || 'N/A'}ms`);

  return {
    'cache-test-results.json': JSON.stringify(data, null, 2),
  };
}
