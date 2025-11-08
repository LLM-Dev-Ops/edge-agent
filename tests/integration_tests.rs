//! Comprehensive End-to-End Integration Tests for LLM Edge Agent
//!
//! Test Coverage:
//! - Smoke Tests (5): Basic functionality validation
//! - Request Flow Tests (10): Complete request lifecycle
//! - Routing Tests (8): All routing strategies and circuit breaker
//! - Provider Tests (6): LLM provider integrations
//! - Cache Tests (5): Multi-tier caching system
//! - Observability Tests (5): Metrics, tracing, logging
//!
//! Total: 39 integration tests targeting >80% coverage

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

mod helpers;
mod mocks;

use helpers::*;
use mocks::*;

// =============================================================================
// SMOKE TESTS (5 tests)
// =============================================================================

#[tokio::test]
async fn smoke_test_server_starts_successfully() {
    let config = TestConfig::default();
    let server = TestServer::new(config).await;

    assert!(server.is_running(), "Server should be running");
    server.shutdown().await;
}

#[tokio::test]
async fn smoke_test_health_endpoint_responds() {
    let server = TestServer::new(TestConfig::default()).await;

    let response = server.get("/health").await;

    assert_eq!(response.status(), 200);
    let health: HealthResponse = response.json().await;
    assert_eq!(health.status, "healthy");
    assert!(!health.version.is_empty());

    server.shutdown().await;
}

#[tokio::test]
async fn smoke_test_metrics_endpoint_works() {
    let server = TestServer::new(TestConfig::default()).await;

    let response = server.get("/metrics").await;

    assert_eq!(response.status(), 200);
    let metrics_text = response.text().await;
    assert!(metrics_text.contains("# TYPE"));
    assert!(metrics_text.contains("# HELP"));

    server.shutdown().await;
}

#[tokio::test]
async fn smoke_test_basic_auth_works() {
    let server = TestServer::new(TestConfig::with_auth()).await;

    // Without auth - should fail
    let response = server.post("/v1/chat/completions")
        .json(&create_test_request("Hello"))
        .send()
        .await;
    assert_eq!(response.status(), 401);

    // With auth - should succeed
    let response = server.post("/v1/chat/completions")
        .bearer_auth("test-api-key")
        .json(&create_test_request("Hello"))
        .send()
        .await;
    assert_eq!(response.status(), 200);

    server.shutdown().await;
}

#[tokio::test]
async fn smoke_test_basic_request_succeeds() {
    let server = TestServer::new(TestConfig::default()).await;
    let mock_provider = MockProvider::new();
    mock_provider.expect_chat_completion()
        .returning(|_| Ok(create_test_response("Hello from provider")));

    let request = create_test_request("What is Rust?");
    let response = server.post("/v1/chat/completions")
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    let chat_response: ChatCompletionResponse = response.json().await;
    assert!(!chat_response.id.is_empty());
    assert!(!chat_response.choices.is_empty());

    server.shutdown().await;
}

// =============================================================================
// REQUEST FLOW TESTS (10 tests)
// =============================================================================

#[tokio::test]
async fn flow_test_complete_with_cache_miss() {
    let server = TestServer::new(TestConfig::default()).await;
    let metrics = server.metrics();

    // First request - should miss cache
    let request = create_test_request("What is 2+2?");
    let response = server.post("/v1/chat/completions")
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 200);

    // Verify metrics
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.cache_misses, 1);
    assert_eq!(snapshot.provider_requests, 1);
    assert_eq!(snapshot.cache_writes, 1);

    server.shutdown().await;
}

#[tokio::test]
async fn flow_test_complete_with_l1_cache_hit() {
    let server = TestServer::new(TestConfig::default()).await;
    let metrics = server.metrics();

    let request = create_test_request("What is 2+2?");

    // First request - cache miss
    let _ = server.post("/v1/chat/completions").json(&request).send().await;

    // Second request - should hit L1 cache
    let start = std::time::Instant::now();
    let response = server.post("/v1/chat/completions").json(&request).send().await;
    let latency = start.elapsed();

    assert_eq!(response.status(), 200);

    // Verify L1 cache hit
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.l1_hits, 1);
    assert_eq!(snapshot.provider_requests, 1); // Only from first request

    // L1 should be sub-millisecond
    assert!(latency.as_micros() < 1000, "L1 latency should be <1ms, got {:?}", latency);

    server.shutdown().await;
}

#[tokio::test]
async fn flow_test_complete_with_l2_cache_hit() {
    let server = TestServer::new(TestConfig::with_l2_cache()).await;
    let cache = server.cache_manager();

    let request = create_test_request("L2 cache test");

    // Store in L2 only (simulate L1 eviction)
    cache.store_l2_only(&request, create_test_response("L2 response")).await;

    // Request should hit L2 and promote to L1
    let response = server.post("/v1/chat/completions").json(&request).send().await;

    assert_eq!(response.status(), 200);

    let snapshot = server.metrics().snapshot();
    assert_eq!(snapshot.l2_hits, 1);
    assert_eq!(snapshot.l1_writes, 1); // Promoted to L1

    server.shutdown().await;
}

#[tokio::test]
async fn flow_test_request_with_invalid_auth() {
    let server = TestServer::new(TestConfig::with_auth()).await;

    let request = create_test_request("Hello");
    let response = server.post("/v1/chat/completions")
        .bearer_auth("invalid-key")
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 401);

    let error: ErrorResponse = response.json().await;
    assert!(error.message.contains("Invalid") || error.message.contains("Unauthorized"));

    server.shutdown().await;
}

#[tokio::test]
async fn flow_test_request_with_rate_limit_exceeded() {
    let config = TestConfig::default()
        .with_rate_limit(5, Duration::from_secs(60)); // 5 requests per minute
    let server = TestServer::new(config).await;

    let request = create_test_request("Rate limit test");

    // Make 5 requests - should succeed
    for _ in 0..5 {
        let response = server.post("/v1/chat/completions").json(&request).send().await;
        assert_eq!(response.status(), 200);
    }

    // 6th request - should be rate limited
    let response = server.post("/v1/chat/completions").json(&request).send().await;
    assert_eq!(response.status(), 429); // Too Many Requests

    server.shutdown().await;
}

#[tokio::test]
async fn flow_test_request_timeout_handling() {
    let config = TestConfig::default()
        .with_request_timeout(Duration::from_millis(100));
    let server = TestServer::new(config).await;

    let mock_provider = MockProvider::new();
    mock_provider.expect_chat_completion()
        .returning(|_| {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok(create_test_response("Too slow"))
        });

    let request = create_test_request("Timeout test");
    let response = server.post("/v1/chat/completions").json(&request).send().await;

    assert_eq!(response.status(), 408); // Request Timeout

    server.shutdown().await;
}

#[tokio::test]
async fn flow_test_large_request_handling() {
    let server = TestServer::new(TestConfig::default()).await;

    // Create ~10MB request
    let large_prompt = "x".repeat(10 * 1024 * 1024);
    let request = create_test_request(&large_prompt);

    let response = server.post("/v1/chat/completions")
        .json(&request)
        .send()
        .await;

    // Should handle gracefully (accept or reject with proper error)
    assert!(response.status().is_success() || response.status() == 413); // 413 = Payload Too Large

    server.shutdown().await;
}

#[tokio::test]
async fn flow_test_concurrent_requests() {
    let server = Arc::new(TestServer::new(TestConfig::default()).await);

    // Launch 100 concurrent requests
    let mut handles = vec![];
    for i in 0..100 {
        let server_clone = server.clone();
        let handle = tokio::spawn(async move {
            let request = create_test_request(&format!("Request {}", i));
            server_clone.post("/v1/chat/completions")
                .json(&request)
                .send()
                .await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(handles).await;

    // All should succeed
    let success_count = results.iter()
        .filter(|r| r.as_ref().unwrap().status().is_success())
        .count();

    assert!(success_count >= 95, "At least 95% should succeed, got {}/100", success_count);

    server.shutdown().await;
}

#[tokio::test]
async fn flow_test_streaming_response() {
    let server = TestServer::new(TestConfig::default()).await;

    let mut request = create_test_request("Stream test");
    request.stream = Some(true);

    let response = server.post("/v1/chat/completions")
        .json(&request)
        .send()
        .await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "text/event-stream");

    // Verify SSE format
    let body = response.text().await;
    assert!(body.contains("data: "));

    server.shutdown().await;
}

#[tokio::test]
async fn flow_test_graceful_degradation() {
    let config = TestConfig::default()
        .with_l2_cache_failure_mode(); // L2 will fail
    let server = TestServer::new(config).await;

    // Should still work with L1 only
    let request = create_test_request("Degradation test");
    let response = server.post("/v1/chat/completions").json(&request).send().await;

    assert_eq!(response.status(), 200);

    // Verify L2 is unhealthy but system works
    let health = server.get("/health/detailed").await.json::<DetailedHealth>().await;
    assert_eq!(health.status, "degraded");
    assert!(health.l1_healthy);
    assert!(!health.l2_healthy);

    server.shutdown().await;
}

// =============================================================================
// ROUTING TESTS (8 tests)
// =============================================================================

#[tokio::test]
async fn routing_test_round_robin_distributes_evenly() {
    let config = TestConfig::default()
        .with_routing_strategy(RoutingStrategy::RoundRobin);
    let server = TestServer::new(config).await;

    // Make 10 requests
    for i in 0..10 {
        let request = create_test_request(&format!("Request {}", i));
        let _ = server.post("/v1/chat/completions").json(&request).send().await;
    }

    // Check provider distribution
    let metrics = server.metrics().snapshot();
    let openai_count = metrics.provider_requests_by_name("openai");
    let anthropic_count = metrics.provider_requests_by_name("anthropic");

    // Should be approximately even (5/5 or 6/4)
    assert!((openai_count as i32 - anthropic_count as i32).abs() <= 1);

    server.shutdown().await;
}

#[tokio::test]
async fn routing_test_failover_uses_priority() {
    let config = TestConfig::default()
        .with_routing_strategy(RoutingStrategy::Failover)
        .with_provider_priorities(vec![
            ("openai", 1),
            ("anthropic", 2),
        ]);
    let server = TestServer::new(config).await;

    // All requests should go to highest priority (openai = 1)
    for i in 0..5 {
        let request = create_test_request(&format!("Request {}", i));
        let _ = server.post("/v1/chat/completions").json(&request).send().await;
    }

    let metrics = server.metrics().snapshot();
    assert_eq!(metrics.provider_requests_by_name("openai"), 5);
    assert_eq!(metrics.provider_requests_by_name("anthropic"), 0);

    server.shutdown().await;
}

#[tokio::test]
async fn routing_test_least_latency_selects_fastest() {
    let config = TestConfig::default()
        .with_routing_strategy(RoutingStrategy::LeastLatency);
    let server = TestServer::new(config).await;

    // Configure mock latencies
    server.set_provider_latency("openai", Duration::from_millis(10));
    server.set_provider_latency("anthropic", Duration::from_millis(50));

    // Make requests - should route to faster provider
    for i in 0..5 {
        let request = create_test_request(&format!("Request {}", i));
        let _ = server.post("/v1/chat/completions").json(&request).send().await;
    }

    let metrics = server.metrics().snapshot();
    assert!(metrics.provider_requests_by_name("openai") >= 4); // At least 80% to faster

    server.shutdown().await;
}

#[tokio::test]
async fn routing_test_cost_optimized_selects_cheapest() {
    let config = TestConfig::default()
        .with_routing_strategy(RoutingStrategy::CostOptimized);
    let server = TestServer::new(config).await;

    // OpenAI: $0.03/1K tokens, Anthropic: $0.025/1K tokens
    // Should prefer Anthropic

    for i in 0..5 {
        let request = create_test_request(&format!("Request {}", i));
        let _ = server.post("/v1/chat/completions").json(&request).send().await;
    }

    let metrics = server.metrics().snapshot();
    assert!(metrics.provider_requests_by_name("anthropic") >= 4); // At least 80% to cheaper

    server.shutdown().await;
}

#[tokio::test]
async fn routing_test_circuit_breaker_opens_on_failures() {
    let config = TestConfig::default()
        .with_circuit_breaker(
            5,  // failure_threshold
            Duration::from_secs(30), // timeout
        );
    let server = TestServer::new(config).await;

    let mock_provider = MockProvider::new();
    mock_provider.expect_chat_completion()
        .times(5)
        .returning(|_| Err(ProviderError::ServiceUnavailable));

    // Make 5 failing requests
    for i in 0..5 {
        let request = create_test_request(&format!("Request {}", i));
        let _ = server.post("/v1/chat/completions").json(&request).send().await;
    }

    // Circuit breaker should be open now
    let cb_state = server.circuit_breaker_state("openai").await;
    assert_eq!(cb_state, CircuitBreakerState::Open);

    server.shutdown().await;
}

#[tokio::test]
async fn routing_test_circuit_breaker_recovers() {
    let config = TestConfig::default()
        .with_circuit_breaker(
            3,  // failure_threshold
            Duration::from_millis(100), // short timeout for testing
        );
    let server = TestServer::new(config).await;

    // Trip circuit breaker
    server.set_provider_failing("openai", true);
    for _ in 0..3 {
        let _ = server.post("/v1/chat/completions")
            .json(&create_test_request("fail"))
            .send()
            .await;
    }

    assert_eq!(server.circuit_breaker_state("openai").await, CircuitBreakerState::Open);

    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Should be half-open
    assert_eq!(server.circuit_breaker_state("openai").await, CircuitBreakerState::HalfOpen);

    // Fix provider and make successful request
    server.set_provider_failing("openai", false);
    let response = server.post("/v1/chat/completions")
        .json(&create_test_request("success"))
        .send()
        .await;
    assert_eq!(response.status(), 200);

    // Should be closed now
    assert_eq!(server.circuit_breaker_state("openai").await, CircuitBreakerState::Closed);

    server.shutdown().await;
}

#[tokio::test]
async fn routing_test_provider_health_affects_routing() {
    let config = TestConfig::default()
        .with_routing_strategy(RoutingStrategy::HealthAware);
    let server = TestServer::new(config).await;

    // Mark one provider unhealthy
    server.set_provider_healthy("openai", false);

    // Requests should go to healthy provider
    for i in 0..5 {
        let request = create_test_request(&format!("Request {}", i));
        let _ = server.post("/v1/chat/completions").json(&request).send().await;
    }

    let metrics = server.metrics().snapshot();
    assert_eq!(metrics.provider_requests_by_name("openai"), 0);
    assert_eq!(metrics.provider_requests_by_name("anthropic"), 5);

    server.shutdown().await;
}

#[tokio::test]
async fn routing_test_fallback_chain_works() {
    let config = TestConfig::default()
        .with_fallback_chain(vec!["openai", "anthropic", "google"]);
    let server = TestServer::new(config).await;

    // First provider fails
    server.set_provider_failing("openai", true);

    let request = create_test_request("Fallback test");
    let response = server.post("/v1/chat/completions").json(&request).send().await;

    assert_eq!(response.status(), 200);

    // Should have used fallback
    let chat_response: ChatCompletionResponse = response.json().await;
    assert_eq!(chat_response.provider, "anthropic");

    server.shutdown().await;
}

// =============================================================================
// PROVIDER TESTS (6 tests)
// =============================================================================

#[tokio::test]
async fn provider_test_openai_request_succeeds() {
    let server = TestServer::new(TestConfig::default()).await;
    let mock = MockOpenAI::new();
    server.register_mock_provider("openai", mock);

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            Message { role: "user".to_string(), content: "Hello".to_string() }
        ],
        temperature: Some(0.7),
        max_tokens: Some(100),
        stream: None,
    };

    let response = server.post("/v1/chat/completions").json(&request).send().await;

    assert_eq!(response.status(), 200);
    let chat_response: ChatCompletionResponse = response.json().await;
    assert_eq!(chat_response.model, "gpt-4");

    server.shutdown().await;
}

#[tokio::test]
async fn provider_test_anthropic_request_succeeds() {
    let server = TestServer::new(TestConfig::default()).await;
    let mock = MockAnthropic::new();
    server.register_mock_provider("anthropic", mock);

    let request = ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![
            Message { role: "user".to_string(), content: "Hello Claude".to_string() }
        ],
        temperature: Some(0.7),
        max_tokens: Some(100),
        stream: None,
    };

    let response = server.post("/v1/chat/completions").json(&request).send().await;

    assert_eq!(response.status(), 200);
    let chat_response: ChatCompletionResponse = response.json().await;
    assert!(chat_response.model.contains("claude"));

    server.shutdown().await;
}

#[tokio::test]
async fn provider_test_retry_on_failure() {
    let config = TestConfig::default()
        .with_retry_policy(3, Duration::from_millis(100)); // 3 retries
    let server = TestServer::new(config).await;

    let mock = MockProvider::new();
    let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let call_count_clone = call_count.clone();

    mock.expect_chat_completion()
        .returning(move |_| {
            let count = call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count < 2 {
                Err(ProviderError::ServiceUnavailable)
            } else {
                Ok(create_test_response("Success on retry"))
            }
        });

    let request = create_test_request("Retry test");
    let response = server.post("/v1/chat/completions").json(&request).send().await;

    assert_eq!(response.status(), 200);
    assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);

    server.shutdown().await;
}

#[tokio::test]
async fn provider_test_timeout_handling() {
    let config = TestConfig::default()
        .with_provider_timeout(Duration::from_millis(100));
    let server = TestServer::new(config).await;

    let mock = MockProvider::new();
    mock.expect_chat_completion()
        .returning(|_| {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok(create_test_response("Too slow"))
        });

    let request = create_test_request("Timeout test");
    let response = server.post("/v1/chat/completions").json(&request).send().await;

    assert!(response.status().is_client_error() || response.status().is_server_error());

    server.shutdown().await;
}

#[tokio::test]
async fn provider_test_invalid_model_handling() {
    let server = TestServer::new(TestConfig::default()).await;

    let request = ChatCompletionRequest {
        model: "invalid-model-xyz".to_string(),
        messages: vec![
            Message { role: "user".to_string(), content: "Hello".to_string() }
        ],
        temperature: None,
        max_tokens: None,
        stream: None,
    };

    let response = server.post("/v1/chat/completions").json(&request).send().await;

    assert_eq!(response.status(), 400); // Bad Request

    server.shutdown().await;
}

#[tokio::test]
async fn provider_test_cost_tracking_works() {
    let server = TestServer::new(TestConfig::default()).await;

    // Make several requests
    for i in 0..5 {
        let request = create_test_request(&format!("Request {}", i));
        let _ = server.post("/v1/chat/completions").json(&request).send().await;
    }

    // Check cost metrics
    let metrics = server.metrics().snapshot();
    assert!(metrics.total_cost > 0.0);
    assert!(metrics.cost_by_provider("openai") > 0.0);

    // Verify cost calculation is reasonable
    // Assuming ~30 tokens per request at $0.03/1K = $0.0009/request
    // 5 requests = ~$0.0045
    assert!(metrics.total_cost > 0.001 && metrics.total_cost < 0.01);

    server.shutdown().await;
}

// =============================================================================
// CACHE TESTS (5 tests)
// =============================================================================

#[tokio::test]
async fn cache_test_l1_stores_and_retrieves() {
    let server = TestServer::new(TestConfig::default()).await;

    let request = create_test_request("Cache test");

    // First request - miss
    let response1 = server.post("/v1/chat/completions").json(&request).send().await;
    assert_eq!(response1.status(), 200);

    // Second request - L1 hit
    let start = std::time::Instant::now();
    let response2 = server.post("/v1/chat/completions").json(&request).send().await;
    let latency = start.elapsed();

    assert_eq!(response2.status(), 200);
    assert!(latency.as_micros() < 1000); // Sub-millisecond

    let metrics = server.metrics().snapshot();
    assert_eq!(metrics.l1_hits, 1);

    server.shutdown().await;
}

#[tokio::test]
async fn cache_test_l2_stores_and_retrieves() {
    let server = TestServer::new(TestConfig::with_l2_cache()).await;
    let cache = server.cache_manager();

    let request = create_test_request("L2 test");
    let response_data = create_test_response("L2 cached response");

    // Store directly in L2
    cache.store_l2_only(&request, response_data).await;

    // Retrieve should hit L2
    let response = server.post("/v1/chat/completions").json(&request).send().await;
    assert_eq!(response.status(), 200);

    let metrics = server.metrics().snapshot();
    assert_eq!(metrics.l2_hits, 1);

    server.shutdown().await;
}

#[tokio::test]
async fn cache_test_invalidation_works() {
    let server = TestServer::new(TestConfig::default()).await;

    let request = create_test_request("Invalidation test");

    // Populate cache
    let _ = server.post("/v1/chat/completions").json(&request).send().await;

    // Verify cached
    let metrics1 = server.metrics().snapshot();
    let _ = server.post("/v1/chat/completions").json(&request).send().await;
    let metrics2 = server.metrics().snapshot();
    assert_eq!(metrics2.l1_hits, metrics1.l1_hits + 1);

    // Invalidate
    let _ = server.post("/v1/cache/invalidate").json(&request).send().await;

    // Should miss now
    let _ = server.post("/v1/chat/completions").json(&request).send().await;
    let metrics3 = server.metrics().snapshot();
    assert_eq!(metrics3.cache_misses, metrics2.cache_misses + 1);

    server.shutdown().await;
}

#[tokio::test]
async fn cache_test_ttl_expiration() {
    let config = TestConfig::default()
        .with_cache_ttl(Duration::from_secs(1));
    let server = TestServer::new(config).await;

    let request = create_test_request("TTL test");

    // Populate cache
    let _ = server.post("/v1/chat/completions").json(&request).send().await;

    // Immediately - should hit
    let response1 = server.post("/v1/chat/completions").json(&request).send().await;
    assert_eq!(response1.status(), 200);
    assert_eq!(server.metrics().snapshot().l1_hits, 1);

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should miss now
    let _ = server.post("/v1/chat/completions").json(&request).send().await;
    let final_metrics = server.metrics().snapshot();
    assert!(final_metrics.cache_misses > 1);

    server.shutdown().await;
}

#[tokio::test]
async fn cache_test_graceful_l2_degradation() {
    let config = TestConfig::with_l2_cache()
        .with_l2_failure_mode();
    let server = TestServer::new(config).await;

    // L2 is failing, but L1 should work
    let request = create_test_request("Degradation test");

    // Should still succeed with L1 only
    let response = server.post("/v1/chat/completions").json(&request).send().await;
    assert_eq!(response.status(), 200);

    // Verify health status
    let health = server.get("/health").await.json::<HealthResponse>().await;
    assert_eq!(health.status, "degraded");

    server.shutdown().await;
}

// =============================================================================
// OBSERVABILITY TESTS (5 tests)
// =============================================================================

#[tokio::test]
async fn observability_test_metrics_recorded_correctly() {
    let server = TestServer::new(TestConfig::default()).await;

    // Generate traffic
    for i in 0..10 {
        let request = create_test_request(&format!("Metrics {}", i));
        let _ = server.post("/v1/chat/completions").json(&request).send().await;
    }

    let metrics = server.get("/metrics").await.text().await;

    // Verify key metrics exist
    assert!(metrics.contains("llm_requests_total"));
    assert!(metrics.contains("llm_cache_hits_total"));
    assert!(metrics.contains("llm_cache_misses_total"));
    assert!(metrics.contains("llm_provider_requests_total"));
    assert!(metrics.contains("llm_request_duration_seconds"));

    server.shutdown().await;
}

#[tokio::test]
async fn observability_test_traces_created_for_requests() {
    let config = TestConfig::default()
        .with_tracing_enabled();
    let server = TestServer::new(config).await;
    let tracer = server.tracer();

    let request = create_test_request("Trace test");
    let _ = server.post("/v1/chat/completions").json(&request).send().await;

    // Verify trace was created
    let traces = tracer.get_traces().await;
    assert!(!traces.is_empty());

    let trace = &traces[0];
    assert!(trace.spans.iter().any(|s| s.name == "http_request"));
    assert!(trace.spans.iter().any(|s| s.name == "cache_lookup"));
    assert!(trace.spans.iter().any(|s| s.name == "provider_request"));

    server.shutdown().await;
}

#[tokio::test]
async fn observability_test_pii_redacted_in_logs() {
    let config = TestConfig::default()
        .with_pii_redaction(true);
    let server = TestServer::new(config).await;
    let log_capture = server.log_capture();

    // Make request with PII
    let request = create_test_request("My email is user@example.com and SSN is 123-45-6789");
    let _ = server.post("/v1/chat/completions").json(&request).send().await;

    // Check logs
    let logs = log_capture.get_logs().await;
    let log_text = logs.join("\n");

    // Should not contain PII
    assert!(!log_text.contains("user@example.com"));
    assert!(!log_text.contains("123-45-6789"));

    // Should contain redaction markers
    assert!(log_text.contains("[REDACTED_EMAIL]") || log_text.contains("[EMAIL]"));
    assert!(log_text.contains("[REDACTED_SSN]") || log_text.contains("[SSN]"));

    server.shutdown().await;
}

#[tokio::test]
async fn observability_test_error_logs_captured() {
    let server = TestServer::new(TestConfig::default()).await;
    let log_capture = server.log_capture();

    // Trigger error
    server.set_provider_failing("openai", true);
    server.set_provider_failing("anthropic", true);

    let request = create_test_request("Error test");
    let _ = server.post("/v1/chat/completions").json(&request).send().await;

    // Check error logs
    let logs = log_capture.get_logs_at_level("ERROR").await;
    assert!(!logs.is_empty());
    assert!(logs.iter().any(|l| l.contains("provider") || l.contains("failed")));

    server.shutdown().await;
}

#[tokio::test]
async fn observability_test_cost_tracking_accurate() {
    let server = TestServer::new(TestConfig::default()).await;

    // Make requests with known token counts
    let mock = MockProvider::new();
    mock.expect_chat_completion()
        .returning(|_| {
            Ok(ChatCompletionResponse {
                id: "test-id".to_string(),
                model: "gpt-4".to_string(),
                choices: vec![Choice {
                    index: 0,
                    message: Message {
                        role: "assistant".to_string(),
                        content: "Response".to_string(),
                    },
                    finish_reason: "stop".to_string(),
                }],
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                }),
                provider: "openai".to_string(),
            })
        });

    // Make 5 requests
    for i in 0..5 {
        let request = create_test_request(&format!("Request {}", i));
        let _ = server.post("/v1/chat/completions").json(&request).send().await;
    }

    // Calculate expected cost
    // 5 requests * 30 tokens * $0.03/1K tokens = $0.0045
    let metrics = server.metrics().snapshot();
    let expected_cost = 5.0 * 30.0 * 0.03 / 1000.0;

    assert!((metrics.total_cost - expected_cost).abs() < 0.0001);

    server.shutdown().await;
}
