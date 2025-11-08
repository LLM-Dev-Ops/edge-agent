# LLM-Edge-Agent System Architecture

## Executive Summary

LLM-Edge-Agent is an intercepting proxy designed for multi-provider LLM APIs, built in Rust for maximum performance and reliability. The system provides intelligent request routing, multi-level caching, request enrichment, and comprehensive logging capabilities.

---

## 1. System Architecture Overview

### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CLIENT LAYER                                │
│  (OpenAI SDK, Anthropic SDK, Custom Clients, HTTP Clients)              │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         EDGE PROXY LAYER                                 │
│ ┌─────────────────────────────────────────────────────────────────────┐ │
│ │                    HTTP/HTTPS Server (Axum)                         │ │
│ │                  (TLS Termination, Connection Pool)                 │ │
│ └─────────────────────────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────────────────────┐ │
│ │                   Request Interception Layer                        │ │
│ │  • Provider Detection  • Authentication  • Request Parsing          │ │
│ └─────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      PROCESSING PIPELINE                                 │
│ ┌───────────────┐    ┌──────────────┐    ┌─────────────────────────┐   │
│ │  Cache Check  │───▶│   Routing    │───▶│   Request Enrichment    │   │
│ │   (L1/L2/L3)  │    │   Engine     │    │ (Metadata, Tracing, etc)│   │
│ └───────────────┘    └──────────────┘    └─────────────────────────┘   │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        PROVIDER LAYER                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │   OpenAI     │  │  Anthropic   │  │    Google    │  │   Custom   │ │
│  │   Adapter    │  │   Adapter    │  │   Adapter    │  │  Adapters  │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └────────────┘ │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      OBSERVABILITY LAYER                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │   Logging    │  │   Metrics    │  │   Tracing    │  │   Cache    │ │
│  │  (Structured)│  │ (Prometheus) │  │  (OpenTel)   │  │   Write    │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        STORAGE LAYER                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │    Redis     │  │  PostgreSQL  │  │   S3/Blob    │  │   Local    │ │
│  │ (L1 Cache)   │  │  (Metadata)  │  │  (Archives)  │  │   Disk     │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Core Design Principles

1. **Zero-Copy Where Possible**: Minimize data copying using Rust's ownership model
2. **Async-First**: Built on Tokio for concurrent request handling
3. **Type-Safe Configuration**: Strong typing for all configurations and policies
4. **Hot-Reloadable**: Configuration changes without service restart
5. **Fail-Fast**: Early validation and error handling
6. **Observable**: Comprehensive metrics, logs, and traces

---

## 2. Request/Response Flow

### 2.1 Complete Request Lifecycle

```
┌──────────────┐
│ Client sends │
│   request    │
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────────────┐
│ Stage 1: Request Interception                │
│ ┌──────────────────────────────────────────┐ │
│ │ 1.1 TLS Termination                      │ │
│ │ 1.2 HTTP/2 or HTTP/1.1 Upgrade           │ │
│ │ 1.3 Connection pooling check             │ │
│ └──────────────────────────────────────────┘ │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│ Stage 2: Provider Detection & Validation     │
│ ┌──────────────────────────────────────────┐ │
│ │ 2.1 Parse request path/headers           │ │
│ │ 2.2 Detect provider (OpenAI, Anthropic)  │ │
│ │ 2.3 Validate API key format              │ │
│ │ 2.4 Extract model identifier             │ │
│ │ 2.5 Parse request body (streaming)       │ │
│ └──────────────────────────────────────────┘ │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│ Stage 3: Cache Lookup (L1 → L2 → L3)        │
│ ┌──────────────────────────────────────────┐ │
│ │ 3.1 Generate cache key (hash of request) │ │
│ │ 3.2 Check L1: In-memory LRU cache        │ │
│ │     ├─ Hit: Return cached response       │ │
│ │     └─ Miss: Continue                    │ │
│ │ 3.3 Check L2: Redis distributed cache    │ │
│ │     ├─ Hit: Update L1, return response   │ │
│ │     └─ Miss: Continue                    │ │
│ │ 3.4 Check L3: Semantic/Embedding cache   │ │
│ │     ├─ Similar: Return with confidence   │ │
│ │     └─ Miss: Continue to routing         │ │
│ └──────────────────────────────────────────┘ │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│ Stage 4: Routing Decision                    │
│ ┌──────────────────────────────────────────┐ │
│ │ 4.1 Evaluate routing policy:             │ │
│ │     • Cost-based routing                 │ │
│ │     • Latency-based routing              │ │
│ │     • Load-based routing                 │ │
│ │     • Custom policy evaluation           │ │
│ │ 4.2 Select target provider/model         │ │
│ │ 4.3 Check rate limits & quotas           │ │
│ │ 4.4 Circuit breaker status               │ │
│ └──────────────────────────────────────────┘ │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│ Stage 5: Request Enrichment                  │
│ ┌──────────────────────────────────────────┐ │
│ │ 5.1 Add tracing headers (trace-id, etc)  │ │
│ │ 5.2 Inject custom metadata               │ │
│ │ 5.3 Transform request for target API     │ │
│ │ 5.4 Add custom system prompts (optional) │ │
│ │ 5.5 Budget tracking headers              │ │
│ └──────────────────────────────────────────┘ │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│ Stage 6: Provider Invocation                 │
│ ┌──────────────────────────────────────────┐ │
│ │ 6.1 Acquire connection from pool         │ │
│ │ 6.2 Send HTTP request to provider        │ │
│ │ 6.3 Handle streaming responses           │ │
│ │ 6.4 Monitor for timeouts                 │ │
│ │ 6.5 Retry logic (if configured)          │ │
│ └──────────────────────────────────────────┘ │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│ Stage 7: Response Processing                 │
│ ┌──────────────────────────────────────────┐ │
│ │ 7.1 Parse provider response              │ │
│ │ 7.2 Extract usage metadata (tokens)      │ │
│ │ 7.3 Transform to standard format         │ │
│ │ 7.4 Calculate cost                       │ │
│ │ 7.5 Update metrics                       │ │
│ └──────────────────────────────────────────┘ │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│ Stage 8: Cache Write & Logging               │
│ ┌──────────────────────────────────────────┐ │
│ │ 8.1 Write to L1 cache (if cacheable)     │ │
│ │ 8.2 Write to L2 cache (async)            │ │
│ │ 8.3 Generate embedding for L3 (async)    │ │
│ │ 8.4 Log request/response pair            │ │
│ │ 8.5 Update usage tracking                │ │
│ │ 8.6 Emit distributed trace               │ │
│ └──────────────────────────────────────────┘ │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────┐
│ Return to    │
│    Client    │
└──────────────┘
```

### 2.2 Streaming Response Handling

For streaming requests (Server-Sent Events):

```
Provider Response Stream
        │
        ▼
┌───────────────────────┐
│ Stream Chunk Received │
└───────┬───────────────┘
        │
        ▼
┌────────────────────────────────┐
│ Parse & Buffer Chunk           │
│ • Validate SSE format          │
│ • Extract delta content        │
│ • Accumulate for cache         │
└───────┬────────────────────────┘
        │
        ▼
┌────────────────────────────────┐
│ Forward to Client Immediately  │
│ (Zero-delay streaming)         │
└───────┬────────────────────────┘
        │
        ▼
┌────────────────────────────────┐
│ On Stream Complete:            │
│ • Write accumulated response   │
│ • Update cache                 │
│ • Log complete exchange        │
└────────────────────────────────┘
```

---

## 3. Caching Layer Design

### 3.1 Three-Tier Caching Strategy

```
┌──────────────────────────────────────────────────────────────┐
│                      L1 Cache: In-Memory LRU                 │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ Technology: moka (Rust concurrent cache)                 │ │
│ │ Size: Configurable (default: 1000 entries)               │ │
│ │ TTL: 5 minutes (configurable)                            │ │
│ │ Eviction: LRU with size and TTL limits                   │ │
│ │ Key: SHA-256(normalized_request)                         │ │
│ │ Value: Serialized response + metadata                    │ │
│ │                                                           │ │
│ │ Advantages:                                              │ │
│ │ ✓ Sub-microsecond latency                                │ │
│ │ ✓ No network overhead                                    │ │
│ │ ✓ Handles burst traffic                                  │ │
│ │                                                           │ │
│ │ Limitations:                                             │ │
│ │ ✗ Process-local only                                     │ │
│ │ ✗ Lost on restart                                        │ │
│ │ ✗ Limited by RAM                                         │ │
│ └──────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│                   L2 Cache: Distributed Redis                │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ Technology: Redis Cluster                                │ │
│ │ Size: Configurable (GB-scale)                            │ │
│ │ TTL: 1 hour (configurable per-request)                   │ │
│ │ Eviction: LRU + TTL                                      │ │
│ │ Key: SHA-256(normalized_request)                         │ │
│ │ Value: Compressed msgpack(response + metadata)           │ │
│ │                                                           │ │
│ │ Advantages:                                              │ │
│ │ ✓ Shared across multiple proxy instances                │ │
│ │ ✓ Persistent across restarts                            │ │
│ │ ✓ Large storage capacity                                │ │
│ │ ✓ Sub-millisecond latency                               │ │
│ │                                                           │ │
│ │ Limitations:                                             │ │
│ │ ✗ Network latency (0.5-2ms)                             │ │
│ │ ✗ Requires separate infrastructure                      │ │
│ └──────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│                L3 Cache: Semantic/Embedding Cache            │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ Technology: Vector DB (Qdrant/Milvus) + Embedding Model  │ │
│ │ Size: Unlimited (disk-backed)                            │ │
│ │ TTL: 24 hours (configurable)                             │ │
│ │ Key: Embedding vector (384-dim)                          │ │
│ │ Value: Original request + response                       │ │
│ │ Similarity Threshold: 0.95 (cosine similarity)           │ │
│ │                                                           │ │
│ │ Process:                                                 │ │
│ │ 1. Generate embedding from user prompt                   │ │
│ │ 2. Query vector DB for similar prompts                   │ │
│ │ 3. If similarity > threshold, return cached response     │ │
│ │ 4. Include confidence score in metadata                  │ │
│ │                                                           │ │
│ │ Advantages:                                              │ │
│ │ ✓ Handles rephrased/similar queries                      │ │
│ │ ✓ Higher cache hit rate                                  │ │
│ │ ✓ Valuable for chatbots with common questions           │ │
│ │                                                           │ │
│ │ Limitations:                                             │ │
│ │ ✗ Higher latency (5-20ms)                               │ │
│ │ ✗ Embedding generation cost                             │ │
│ │ ✗ False positives risk                                  │ │
│ │ ✗ Complex infrastructure                                │ │
│ └──────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 Cache Key Generation

```rust
// Pseudocode for cache key generation
fn generate_cache_key(request: &LLMRequest) -> CacheKey {
    let normalized = NormalizedRequest {
        provider: request.provider.canonical_name(),
        model: request.model,
        messages: normalize_messages(&request.messages),
        temperature: normalize_float(request.temperature),
        max_tokens: request.max_tokens,
        // Exclude: stream, user, metadata, trace_id
    };

    let hash = blake3::hash(&bincode::serialize(&normalized));
    CacheKey::from(hash)
}
```

### 3.3 Cacheability Rules

```
┌─────────────────────────────────────────────────────────┐
│                 Cacheability Decision Tree              │
└─────────────────────────────────────────────────────────┘

Is temperature = 0?
    ├─ No ─> NOT CACHEABLE (non-deterministic)
    └─ Yes
        │
        Is request marked as cache=false?
            ├─ Yes ─> NOT CACHEABLE (explicit opt-out)
            └─ No
                │
                Is response an error?
                    ├─ Yes ─> NOT CACHEABLE (errors)
                    └─ No
                        │
                        Is response complete? (finish_reason)
                            ├─ No ─> NOT CACHEABLE (truncated)
                            └─ Yes
                                │
                                Is request size > 100KB?
                                    ├─ Yes ─> CACHE L2/L3 only
                                    └─ No ─> CACHE ALL LAYERS

Cache TTL Selection:
    • User-specified: Use request.cache_ttl
    • Deterministic (temp=0): 1 hour
    • Embeddings/Classifications: 24 hours
    • Chat completions: 5 minutes
```

### 3.4 Token-Level Caching (Advanced)

For providers supporting prompt caching (Anthropic):

```
┌──────────────────────────────────────────────────────────┐
│              Token-Level Cache Strategy                  │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  Request with long system prompt:                       │
│  ┌────────────────────────────────────────────────┐     │
│  │ System: [10K tokens] - CACHEABLE               │     │
│  │ User: "What is AI?" - NOT CACHED               │     │
│  └────────────────────────────────────────────────┘     │
│                                                          │
│  Detection:                                              │
│  • Identify static prefix messages                      │
│  • Mark with cache_control: { type: "ephemeral" }      │
│  • Provider handles server-side caching                 │
│                                                          │
│  Benefits:                                               │
│  • Reduce token costs by 90% for cached tokens          │
│  • Lower latency for repeated prefixes                  │
│  • Automatic by proxy (transparent to client)           │
└──────────────────────────────────────────────────────────┘
```

### 3.5 Cache Trade-offs Matrix

```
┌────────────┬──────────┬──────────┬──────────┬─────────────┬──────────┐
│ Cache Type │ Latency  │ Hit Rate │   Cost   │ Complexity  │  Scale   │
├────────────┼──────────┼──────────┼──────────┼─────────────┼──────────┤
│ L1 Memory  │ 0.1ms    │ 30-40%   │ Free     │ Low         │ Limited  │
│ L2 Redis   │ 1-2ms    │ 60-70%   │ Low      │ Medium      │ High     │
│ L3 Vector  │ 10-20ms  │ 80-85%   │ Medium   │ High        │ Unlimited│
│ Token      │ API-side │ Varies   │ -90%     │ Low         │ API-side │
└────────────┴──────────┴──────────┴──────────┴─────────────┴──────────┘

Recommendation:
• Start with L1 + L2 for most use cases
• Add L3 for FAQ/support chatbots
• Use token caching for long system prompts
```

---

## 4. Routing Decision Engine

### 4.1 Routing Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Routing Decision Engine                  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           Policy Configuration Layer                │   │
│  │  • YAML/TOML config with hot-reload                 │   │
│  │  • Per-tenant routing rules                         │   │
│  │  • Time-based routing (peak/off-peak)               │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                  │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Routing Strategies                     │   │
│  │                                                      │   │
│  │  1. Cost-Based Routing                              │   │
│  │     • Select cheapest provider for model class      │   │
│  │     • Consider input/output token pricing           │   │
│  │     • Account for cache hits (free)                 │   │
│  │                                                      │   │
│  │  2. Latency-Based Routing                           │   │
│  │     • Track P50/P95/P99 latencies per provider      │   │
│  │     • Route to fastest provider                     │   │
│  │     • Consider geographic proximity                 │   │
│  │                                                      │   │
│  │  3. Load-Based Routing                              │   │
│  │     • Monitor provider error rates                  │   │
│  │     • Circuit breaker pattern                       │   │
│  │     • Fallback to alternative providers             │   │
│  │                                                      │   │
│  │  4. Quality-Based Routing                           │   │
│  │     • Track response quality metrics                │   │
│  │     • User feedback scores                          │   │
│  │     • Model benchmarks                              │   │
│  │                                                      │   │
│  │  5. Custom Policy Routing                           │   │
│  │     • Lua/WASM scripts for custom logic             │   │
│  │     • A/B testing frameworks                        │   │
│  │     • Compliance requirements (data residency)      │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                  │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           Decision Execution Layer                  │   │
│  │  • Evaluate rules in priority order                 │   │
│  │  • Apply constraints (budget, quotas)               │   │
│  │  • Select target provider + model                   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 Routing Policy Examples

#### Cost-Based Routing

```yaml
routing:
  strategy: cost_optimized
  policies:
    - name: "cheap-gpt4-equivalent"
      match:
        model_class: "gpt-4"
      route:
        - provider: "anthropic"
          model: "claude-3-sonnet-20240229"
          weight: 70  # Prefer 70% of traffic
          max_cost_per_1k_tokens: 0.003
        - provider: "openai"
          model: "gpt-4-turbo-preview"
          weight: 30
          max_cost_per_1k_tokens: 0.01
      fallback:
        - provider: "openai"
          model: "gpt-3.5-turbo"
```

#### Latency-Based Routing

```yaml
routing:
  strategy: latency_optimized
  policies:
    - name: "low-latency-chat"
      match:
        endpoint: "/v1/chat/completions"
        max_latency_ms: 500
      route:
        selection_method: "fastest_p95"
        candidates:
          - provider: "openai"
            region: "us-east-1"
          - provider: "anthropic"
            region: "us-east-1"
          - provider: "groq"
            model: "llama-3-70b"
      monitoring:
        sample_rate: 0.1  # Sample 10% for latency tracking
        window: "5m"
```

#### Load-Based Routing with Circuit Breaker

```yaml
routing:
  strategy: reliability_first
  policies:
    - name: "resilient-routing"
      circuit_breaker:
        failure_threshold: 0.5  # 50% error rate
        timeout: 30s
        half_open_requests: 3
      route:
        - provider: "openai"
          model: "gpt-4"
          priority: 1
        - provider: "anthropic"
          model: "claude-3-opus"
          priority: 2  # Fallback
        - provider: "google"
          model: "gemini-pro"
          priority: 3  # Last resort
```

### 4.3 Routing Decision Algorithm

```rust
// Pseudocode for routing decision
async fn route_request(
    request: &LLMRequest,
    policies: &[RoutingPolicy],
    metrics: &MetricsStore,
) -> RoutingDecision {
    // 1. Find matching policies
    let applicable = policies
        .iter()
        .filter(|p| p.matches(request))
        .collect::<Vec<_>>();

    // 2. Evaluate each candidate
    let mut candidates = vec![];
    for policy in applicable {
        for target in &policy.targets {
            let score = calculate_score(target, policy.strategy, metrics).await;
            candidates.push(ScoredTarget { target, score });
        }
    }

    // 3. Filter by constraints
    candidates.retain(|c| {
        c.target.circuit_breaker.is_closed() &&
        c.target.rate_limiter.has_capacity() &&
        c.target.budget.has_remaining()
    });

    // 4. Select based on weights/scores
    let selected = match policy.selection_method {
        SelectionMethod::Weighted => weighted_random(&candidates),
        SelectionMethod::BestScore => candidates.iter().max_by_key(|c| c.score),
        SelectionMethod::RoundRobin => round_robin(&candidates),
    };

    // 5. Return decision with fallback chain
    RoutingDecision {
        primary: selected,
        fallbacks: candidates.iter().filter(|c| c != selected).collect(),
    }
}
```

### 4.4 Model Equivalence Classes

```
┌──────────────────────────────────────────────────────────┐
│            Model Equivalence Mapping                     │
├──────────────────────────────────────────────────────────┤
│                                                          │
│ Class: "gpt-4-level"                                     │
│   • OpenAI: gpt-4, gpt-4-turbo                          │
│   • Anthropic: claude-3-opus, claude-3-sonnet           │
│   • Google: gemini-1.5-pro                              │
│                                                          │
│ Class: "gpt-3.5-level"                                   │
│   • OpenAI: gpt-3.5-turbo                               │
│   • Anthropic: claude-3-haiku                           │
│   • Google: gemini-1.5-flash                            │
│   • Groq: llama-3-70b                                   │
│                                                          │
│ Class: "code-specialized"                                │
│   • OpenAI: gpt-4-turbo (code optimized)                │
│   • Anthropic: claude-3-opus                            │
│                                                          │
│ Usage: Allows transparent model switching based on      │
│        routing policies while maintaining quality        │
└──────────────────────────────────────────────────────────┘
```

---

## 5. Component Architecture (Rust)

### 5.1 Module Structure

```
llm-edge-agent/
├── Cargo.toml
├── src/
│   ├── main.rs                    # Entry point, server initialization
│   ├── lib.rs                     # Public API exports
│   │
│   ├── server/                    # HTTP server layer
│   │   ├── mod.rs
│   │   ├── handler.rs             # Request handlers
│   │   ├── middleware.rs          # Auth, logging, tracing middleware
│   │   └── tls.rs                 # TLS configuration
│   │
│   ├── proxy/                     # Core proxy logic
│   │   ├── mod.rs
│   │   ├── interceptor.rs         # Request interception
│   │   ├── router.rs              # Routing engine
│   │   └── enricher.rs            # Request enrichment
│   │
│   ├── providers/                 # Provider adapters
│   │   ├── mod.rs
│   │   ├── traits.rs              # Provider trait definitions
│   │   ├── openai.rs              # OpenAI adapter
│   │   ├── anthropic.rs           # Anthropic adapter
│   │   ├── google.rs              # Google adapter
│   │   └── registry.rs            # Provider registry
│   │
│   ├── cache/                     # Caching layers
│   │   ├── mod.rs
│   │   ├── l1_memory.rs           # In-memory LRU cache
│   │   ├── l2_redis.rs            # Redis distributed cache
│   │   ├── l3_semantic.rs         # Semantic/vector cache
│   │   ├── key_gen.rs             # Cache key generation
│   │   └── token_cache.rs         # Token-level caching
│   │
│   ├── routing/                   # Routing decision engine
│   │   ├── mod.rs
│   │   ├── policies.rs            # Policy definitions
│   │   ├── cost_based.rs          # Cost-based routing
│   │   ├── latency_based.rs       # Latency-based routing
│   │   ├── load_based.rs          # Load-based routing
│   │   ├── circuit_breaker.rs     # Circuit breaker pattern
│   │   └── evaluator.rs           # Policy evaluation engine
│   │
│   ├── observability/             # Logging, metrics, tracing
│   │   ├── mod.rs
│   │   ├── logging.rs             # Structured logging (tracing)
│   │   ├── metrics.rs             # Prometheus metrics
│   │   ├── tracing.rs             # OpenTelemetry integration
│   │   └── events.rs              # Event emission
│   │
│   ├── models/                    # Data models
│   │   ├── mod.rs
│   │   ├── request.rs             # Request structures
│   │   ├── response.rs            # Response structures
│   │   ├── config.rs              # Configuration structures
│   │   └── metrics.rs             # Metrics structures
│   │
│   ├── storage/                   # Persistence layer
│   │   ├── mod.rs
│   │   ├── postgres.rs            # PostgreSQL for metadata
│   │   ├── redis.rs               # Redis client
│   │   └── s3.rs                  # S3/blob storage
│   │
│   └── utils/                     # Utilities
│       ├── mod.rs
│       ├── hashing.rs             # Hash functions
│       ├── serialization.rs       # Serde helpers
│       └── time.rs                # Time utilities
│
├── tests/                         # Integration tests
│   ├── e2e_tests.rs
│   ├── cache_tests.rs
│   └── routing_tests.rs
│
└── benches/                       # Performance benchmarks
    └── routing_bench.rs
```

### 5.2 Core Traits

```rust
// Provider trait for multi-provider support
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Provider identifier (e.g., "openai", "anthropic")
    fn name(&self) -> &str;

    /// Detect if request is for this provider
    fn matches(&self, request: &HttpRequest) -> bool;

    /// Transform generic request to provider-specific format
    async fn transform_request(&self, request: UnifiedRequest) -> Result<ProviderRequest>;

    /// Send request to provider
    async fn send(&self, request: ProviderRequest) -> Result<ProviderResponse>;

    /// Transform provider response to generic format
    async fn transform_response(&self, response: ProviderResponse) -> Result<UnifiedResponse>;

    /// Get pricing info for cost calculation
    fn get_pricing(&self, model: &str) -> Option<PricingInfo>;

    /// Health check
    async fn health(&self) -> HealthStatus;
}

// Cache trait for pluggable caching backends
#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &CacheKey) -> Result<Option<CachedResponse>>;
    async fn set(&self, key: CacheKey, value: CachedResponse, ttl: Duration) -> Result<()>;
    async fn invalidate(&self, key: &CacheKey) -> Result<()>;
    async fn clear(&self) -> Result<()>;
    fn stats(&self) -> CacheStats;
}

// Routing policy trait
#[async_trait]
pub trait RoutingPolicy: Send + Sync {
    fn name(&self) -> &str;
    fn matches(&self, request: &UnifiedRequest) -> bool;
    async fn evaluate(&self, candidates: &[Provider], metrics: &MetricsStore) -> RoutingDecision;
}
```

### 5.3 Key Data Structures

```rust
/// Unified request format (provider-agnostic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedRequest {
    pub id: RequestId,
    pub provider: ProviderType,
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub metadata: HashMap<String, Value>,
    pub cache_config: CacheConfig,
}

/// Cache key for deterministic lookups
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    hash: [u8; 32],  // BLAKE3 hash
}

/// Routing decision with fallback chain
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub primary: TargetProvider,
    pub fallbacks: Vec<TargetProvider>,
    pub reason: RoutingReason,
    pub estimated_cost: f64,
    pub estimated_latency: Duration,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub timeout: Duration,
    pub max_retries: u32,
    pub rate_limit: RateLimitConfig,
}
```

### 5.4 Concurrency Model

```
┌─────────────────────────────────────────────────────────┐
│                 Tokio Runtime (Multi-threaded)          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Request Handler Tasks (per-request)                   │
│  ┌────────────────────────────────────────────────┐    │
│  │ async fn handle_request() {                    │    │
│  │   let cache_hit = cache.get().await;           │    │
│  │   if cache_hit.is_some() { return; }           │    │
│  │                                                 │    │
│  │   let route = router.route().await;            │    │
│  │   let response = provider.send().await;        │    │
│  │                                                 │    │
│  │   // Fire-and-forget background tasks          │    │
│  │   tokio::spawn(cache.set(response));           │    │
│  │   tokio::spawn(log_request(request));          │    │
│  │ }                                               │    │
│  └────────────────────────────────────────────────┘    │
│                                                         │
│  Background Tasks (periodic)                            │
│  ┌────────────────────────────────────────────────┐    │
│  │ • Metrics aggregation (every 10s)              │    │
│  │ • Cache eviction (every 60s)                   │    │
│  │ • Health checks (every 30s)                    │    │
│  │ • Config reload (on file change)               │    │
│  └────────────────────────────────────────────────┘    │
│                                                         │
│  Shared State (Arc + RwLock/Mutex)                     │
│  ┌────────────────────────────────────────────────┐    │
│  │ • ProviderRegistry: Arc<ProviderRegistry>      │    │
│  │ • Config: Arc<RwLock<Config>>                  │    │
│  │ • Metrics: Arc<MetricsStore>                   │    │
│  │ • L1 Cache: Arc<MemoryCache>                   │    │
│  └────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

Performance characteristics:
• Request handling: O(1) task spawning
• Shared state: Lock-free reads (Arc), minimal lock contention
• Background work: Non-blocking, doesn't impact request latency
```

---

## 6. Scalability & Performance Considerations

### 6.1 Horizontal Scaling Architecture

```
                    ┌──────────────────┐
                    │  Load Balancer   │
                    │  (HAProxy/NGINX) │
                    └────────┬─────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
    ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
    │ Proxy Node 1 │ │ Proxy Node 2 │ │ Proxy Node N │
    └──────┬───────┘ └──────┬───────┘ └──────┬───────┘
           │                │                │
           └────────────────┼────────────────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
              ▼                           ▼
    ┌──────────────────┐        ┌──────────────────┐
    │  Redis Cluster   │        │   PostgreSQL     │
    │  (Shared Cache)  │        │   (Metadata)     │
    └──────────────────┘        └──────────────────┘

Scaling properties:
• Stateless proxy nodes (easy horizontal scaling)
• Shared cache layer (consistent cache hits)
• Connection pooling per node
• No inter-node communication required
```

### 6.2 Performance Optimizations

#### 6.2.1 Zero-Copy Request Forwarding

```rust
// Use Bytes for zero-copy body handling
async fn forward_request(req: Request<Body>) -> Result<Response<Body>> {
    let (parts, body) = req.into_parts();

    // Stream body directly without buffering
    let stream = body
        .map_ok(|chunk| chunk)  // Zero-copy forwarding
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

    let new_body = Body::wrap_stream(stream);
    let new_req = Request::from_parts(parts, new_body);

    client.request(new_req).await
}
```

#### 6.2.2 Connection Pooling

```rust
// HTTP client with connection pooling
let client = reqwest::Client::builder()
    .pool_max_idle_per_host(100)
    .pool_idle_timeout(Duration::from_secs(90))
    .tcp_keepalive(Duration::from_secs(60))
    .http2_keep_alive_interval(Duration::from_secs(30))
    .http2_keep_alive_timeout(Duration::from_secs(10))
    .build()?;
```

#### 6.2.3 Async Cache Writes

```rust
// Don't block request on cache write
let cache = cache.clone();
let request = request.clone();
let response = response.clone();

tokio::spawn(async move {
    if let Err(e) = cache.set(request, response).await {
        warn!("Cache write failed: {}", e);
    }
});
```

### 6.3 Resource Limits

```yaml
limits:
  # Per-node limits
  max_concurrent_requests: 10000
  max_request_size: 10_485_760  # 10 MB
  max_response_size: 104_857_600  # 100 MB

  # Timeouts
  request_timeout: 60s
  idle_connection_timeout: 90s

  # Memory
  l1_cache_max_size: 1GB
  max_body_buffer: 10MB

  # Rate limiting (per client)
  rate_limit:
    requests_per_second: 100
    burst: 200
```

### 6.4 Performance Benchmarks (Target)

```
┌──────────────────────────────────────────────────────────┐
│              Performance Targets                         │
├──────────────────────────────────────────────────────────┤
│                                                          │
│ Latency Overhead (vs. direct API call):                 │
│   • L1 cache hit: +0.1ms (P99)                          │
│   • L2 cache hit: +2ms (P99)                            │
│   • Cache miss: +5ms (P99)                              │
│                                                          │
│ Throughput (single node):                                │
│   • Non-streaming: 5,000 req/s                          │
│   • Streaming: 2,000 concurrent streams                 │
│                                                          │
│ Cache Hit Rates:                                         │
│   • L1: 30-40%                                          │
│   • L2: 60-70%                                          │
│   • L3: 80-85% (with semantic cache)                    │
│                                                          │
│ Resource Usage (per node):                               │
│   • CPU: <50% at 1000 req/s                             │
│   • Memory: ~2GB (with 1GB L1 cache)                    │
│   • Network: Depends on traffic volume                  │
└──────────────────────────────────────────────────────────┘
```

### 6.5 Monitoring & Alerting

```yaml
metrics:
  # Request metrics
  - request_total (counter)
  - request_duration_seconds (histogram)
  - request_size_bytes (histogram)
  - response_size_bytes (histogram)

  # Cache metrics
  - cache_hits_total{layer="l1|l2|l3"} (counter)
  - cache_misses_total{layer="l1|l2|l3"} (counter)
  - cache_evictions_total (counter)

  # Routing metrics
  - routing_decision_duration_seconds (histogram)
  - provider_requests_total{provider, model} (counter)
  - provider_errors_total{provider, error_type} (counter)

  # Cost metrics
  - estimated_cost_usd_total{provider, model} (counter)
  - tokens_processed_total{provider, model, type="input|output"} (counter)

alerts:
  - name: HighErrorRate
    condition: rate(provider_errors_total[5m]) > 0.1
    severity: warning

  - name: CacheDegradation
    condition: rate(cache_hits_total[5m]) / rate(request_total[5m]) < 0.3
    severity: info

  - name: HighLatency
    condition: histogram_quantile(0.95, request_duration_seconds) > 1.0
    severity: warning
```

---

## 7. Security Considerations

### 7.1 Authentication & Authorization

```
┌──────────────────────────────────────────────────────────┐
│              Security Architecture                       │
├──────────────────────────────────────────────────────────┤
│                                                          │
│ Layer 1: Client Authentication                          │
│   • API Key validation (bearer token)                   │
│   • JWT token support                                   │
│   • mTLS (mutual TLS) for high-security environments    │
│                                                          │
│ Layer 2: Provider API Key Management                    │
│   • Secrets stored in encrypted vault (HashiCorp Vault) │
│   • API keys never logged                               │
│   • Rotation support                                    │
│                                                          │
│ Layer 3: Request/Response Filtering                     │
│   • PII detection and redaction (optional)              │
│   • Sensitive content filtering                         │
│   • Request validation (schema, size limits)            │
│                                                          │
│ Layer 4: Audit Logging                                  │
│   • All requests logged with client identity            │
│   • Tamper-proof audit trail                            │
│   • GDPR compliance support                             │
└──────────────────────────────────────────────────────────┘
```

### 7.2 Data Privacy

```yaml
privacy:
  # Logging controls
  log_request_bodies: false  # Disable for PII compliance
  log_response_bodies: false
  log_api_keys: false  # Always false

  # PII redaction
  pii_detection:
    enabled: true
    patterns:
      - email_addresses
      - phone_numbers
      - ssn
      - credit_cards
    action: redact  # or "hash" or "drop"

  # Data retention
  retention:
    request_logs: 30d
    cache_data: 24h
    metrics: 90d
```

---

## 8. Deployment Architecture

### 8.1 Production Deployment

```
┌─────────────────────────────────────────────────────────────┐
│                    Kubernetes Deployment                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Ingress (NGINX)                                            │
│    ├─ TLS termination                                       │
│    ├─ Rate limiting                                         │
│    └─ Path-based routing                                    │
│                   │                                         │
│                   ▼                                         │
│  Service (ClusterIP)                                        │
│    └─ Load balancing across pods                           │
│                   │                                         │
│                   ▼                                         │
│  Deployment (llm-edge-agent)                                │
│    ├─ Replicas: 3 (auto-scaling 3-10)                      │
│    ├─ Resources:                                            │
│    │   ├─ CPU: 1 core (request), 2 cores (limit)           │
│    │   └─ Memory: 2GB (request), 4GB (limit)               │
│    ├─ Health checks:                                        │
│    │   ├─ Liveness: /health                                │
│    │   └─ Readiness: /ready                                │
│    └─ Config: ConfigMap + Secrets                          │
│                                                             │
│  StatefulSet (Redis)                                        │
│    ├─ Replicas: 3 (cluster mode)                           │
│    └─ Persistent storage                                   │
│                                                             │
│  External Services                                          │
│    ├─ PostgreSQL (managed service)                         │
│    ├─ Vector DB (Qdrant cloud)                             │
│    └─ Object storage (S3)                                  │
└─────────────────────────────────────────────────────────────┘
```

### 8.2 Docker Compose (Development)

```yaml
version: '3.8'
services:
  proxy:
    build: .
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - CONFIG_PATH=/etc/llm-edge-agent/config.yaml
    volumes:
      - ./config:/etc/llm-edge-agent
    depends_on:
      - redis
      - postgres

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: redis-server --maxmemory 1gb --maxmemory-policy allkeys-lru

  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_DB: llm_edge_agent
      POSTGRES_USER: agent
      POSTGRES_PASSWORD: changeme
    ports:
      - "5432:5432"

  prometheus:
    image: prom/prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
```

---

## 9. Configuration Example

```yaml
# config.yaml - Main configuration file

server:
  host: 0.0.0.0
  port: 8080
  tls:
    enabled: true
    cert_path: /etc/certs/tls.crt
    key_path: /etc/certs/tls.key

  limits:
    max_concurrent_requests: 10000
    request_timeout: 60s
    max_request_size: 10MB

providers:
  - name: openai
    api_key: ${OPENAI_API_KEY}
    base_url: https://api.openai.com/v1
    timeout: 60s
    retry:
      max_attempts: 3
      backoff: exponential

  - name: anthropic
    api_key: ${ANTHROPIC_API_KEY}
    base_url: https://api.anthropic.com/v1
    timeout: 60s

  - name: google
    api_key: ${GOOGLE_API_KEY}
    base_url: https://generativelanguage.googleapis.com/v1
    timeout: 60s

cache:
  l1:
    enabled: true
    max_size: 1000
    ttl: 5m

  l2:
    enabled: true
    backend: redis
    redis:
      url: redis://localhost:6379
      pool_size: 20
    ttl: 1h

  l3:
    enabled: false
    backend: qdrant
    qdrant:
      url: http://localhost:6333
      collection: llm_cache
    embedding_model: all-MiniLM-L6-v2
    similarity_threshold: 0.95

routing:
  default_strategy: cost_optimized

  policies:
    - name: production-gpt4
      match:
        model: gpt-4
      strategy: reliability_first
      targets:
        - provider: openai
          model: gpt-4-turbo-preview
          weight: 80
        - provider: anthropic
          model: claude-3-opus-20240229
          weight: 20

    - name: cost-optimized-chat
      match:
        endpoint: /v1/chat/completions
        model: gpt-3.5-turbo
      strategy: cost_optimized
      targets:
        - provider: groq
          model: llama-3-70b-8192
          weight: 70
        - provider: openai
          model: gpt-3.5-turbo
          weight: 30

observability:
  logging:
    level: info
    format: json
    output: stdout

  metrics:
    enabled: true
    endpoint: /metrics
    prometheus: true

  tracing:
    enabled: true
    exporter: otlp
    endpoint: http://jaeger:4317

storage:
  postgres:
    url: ${DATABASE_URL}
    pool_size: 20

  s3:
    bucket: llm-edge-agent-logs
    region: us-east-1
    endpoint: ${S3_ENDPOINT}
```

---

## 10. Future Enhancements

### 10.1 Roadmap

```
Phase 1 (MVP):
  ✓ Basic proxy with OpenAI/Anthropic support
  ✓ L1 + L2 caching
  ✓ Simple cost-based routing
  ✓ Structured logging

Phase 2 (Production-ready):
  • L3 semantic caching
  • Advanced routing policies
  • Circuit breakers
  • Comprehensive metrics
  • Multi-tenancy support

Phase 3 (Advanced):
  • Token-level caching
  • Custom model adapters (WASM)
  • A/B testing framework
  • Real-time analytics dashboard
  • Cost optimization recommendations

Phase 4 (Enterprise):
  • Multi-region deployment
  • Data residency controls
  • Advanced security (SSO, RBAC)
  • SLA enforcement
  • Compliance certifications
```

### 10.2 Potential Optimizations

1. **Request Batching**: Batch multiple small requests to same provider
2. **Speculative Caching**: Pre-cache common follow-up queries
3. **Model Cascading**: Start with fast/cheap model, escalate if needed
4. **Smart Retries**: Retry with different provider on failure
5. **Cost Prediction**: ML model to predict request cost before routing

---

## Conclusion

This architecture provides a robust, scalable foundation for an LLM edge proxy. Key benefits:

- **Performance**: Sub-5ms overhead with multi-tier caching
- **Cost Efficiency**: 50-80% cost reduction through caching and smart routing
- **Reliability**: Circuit breakers, fallbacks, and health checks
- **Observability**: Comprehensive metrics and tracing
- **Flexibility**: Hot-reloadable config, pluggable providers

The Rust implementation ensures memory safety, high performance, and low resource usage, making it suitable for high-throughput production environments.
