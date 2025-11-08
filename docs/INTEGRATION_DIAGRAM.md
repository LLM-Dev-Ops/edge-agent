# LLM Edge Agent - Integration Architecture Diagrams

## System Architecture

```
┌────────────────────────────────────────────────────────────────────────────┐
│                         LLM Edge Agent System                               │
│                              (Port 8080)                                    │
└────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ HTTP Request
                                    ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                          Layer 1: HTTP Server                              │
│                              (Axum Framework)                              │
├────────────────────────────────────────────────────────────────────────────┤
│  • Routes: /v1/chat/completions, /health, /metrics                        │
│  • Authentication & Authorization (future)                                 │
│  • Rate Limiting (future)                                                  │
│  • Request Validation                                                      │
└────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ ChatCompletionRequest
                                    ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                      Layer 2: Proxy Handler                                │
│                      (proxy.rs - handle_chat_completions)                  │
├────────────────────────────────────────────────────────────────────────────┤
│  1. validate_request()              ┌─────────────────┐                   │
│  2. convert_to_cacheable()          │  Observability  │                   │
│  3. cache_manager.lookup()          │  (Cross-cutting)│                   │
│  4. select_provider()               ├─────────────────┤                   │
│  5. convert_to_unified()            │ • Metrics       │                   │
│  6. provider.send()                 │ • Tracing       │                   │
│  7. calculate_cost()                │ • Logging       │                   │
│  8. cache_manager.store() [async]   │ • Cost tracking │                   │
│  9. build_response()                └─────────────────┘                   │
└────────────────────────────────────────────────────────────────────────────┘
         │                   │                    │
         │ Cache lookup      │ Routing decision   │ Provider request
         ▼                   ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌────────────────────────┐
│  Layer 2:     │   │  Layer 2:     │   │  Layer 3:              │
│  Cache        │   │  Routing      │   │  Provider Adapters     │
├───────────────┤   ├───────────────┤   ├────────────────────────┤
│ L1: Moka      │   │ • Model-based │   │ OpenAI Adapter         │
│   <1ms        │   │ • Cost-based  │   │  ├─ GPT-4             │
│   (in-memory) │   │ • Latency     │   │  ├─ GPT-3.5           │
│               │   │ • Hybrid      │   │  └─ o1                │
│ L2: Redis     │   │               │   │                        │
│   1-2ms       │   │ Circuit       │   │ Anthropic Adapter      │
│   (shared)    │   │ Breakers      │   │  ├─ Claude 3.5 Sonnet │
│               │   │               │   │  ├─ Claude 3 Opus     │
└───────────────┘   └───────────────┘   │  └─ Claude 3 Haiku    │
                                        │                        │
                                        │ Future: Gemini, Bedrock│
                                        └────────────────────────┘
                                                 │
                                                 │ HTTP Request
                                                 ▼
                                        ┌────────────────────────┐
                                        │  External LLM APIs     │
                                        ├────────────────────────┤
                                        │ • api.openai.com       │
                                        │ • api.anthropic.com    │
                                        │ • generativelanguage   │
                                        │   .googleapis.com      │
                                        └────────────────────────┘
```

## Request Flow Sequence Diagram

```
Client          Server         Cache          Routing        Provider       External
  │               │              │               │              │            LLM API
  │               │              │               │              │               │
  │ POST /v1/chat/completions    │               │              │               │
  ├──────────────>│              │               │              │               │
  │               │              │               │              │               │
  │               │ 1. Validate  │               │              │               │
  │               │──┐           │               │              │               │
  │               │<─┘           │               │              │               │
  │               │              │               │              │               │
  │               │ 2. Lookup    │               │              │               │
  │               ├─────────────>│               │              │               │
  │               │              │               │              │               │
  │               │    L1 Check  │               │              │               │
  │               │<─────────────┤               │              │               │
  │               │    MISS      │               │              │               │
  │               │              │               │              │               │
  │               │    L2 Check  │               │              │               │
  │               │<─────────────┤               │              │               │
  │               │    MISS      │               │              │               │
  │               │              │               │              │               │
  │               │ 3. Route     │               │              │               │
  │               ├──────────────────────────────>│              │               │
  │               │              │               │              │               │
  │               │              │          Select Provider     │               │
  │               │<─────────────────────────────┤              │               │
  │               │              │               │              │               │
  │               │ 4. Send Request               │              │               │
  │               ├───────────────────────────────────────────>  │               │
  │               │              │               │              │               │
  │               │              │               │              │ HTTP Request  │
  │               │              │               │              ├──────────────>│
  │               │              │               │              │               │
  │               │              │               │              │   Response    │
  │               │              │               │              │<──────────────┤
  │               │              │               │              │               │
  │               │              │               │   UnifiedResponse             │
  │               │<──────────────────────────────────────────── │               │
  │               │              │               │              │               │
  │               │ 5. Calculate Cost             │              │               │
  │               │──┐           │               │              │               │
  │               │<─┘           │               │              │               │
  │               │              │               │              │               │
  │               │ 6. Store (async)              │              │               │
  │               ├─────────────>│               │              │               │
  │               │              │               │              │               │
  │               │              │ Write L1      │              │               │
  │               │              │ Write L2      │              │               │
  │               │              │ (background)  │              │               │
  │               │              │               │              │               │
  │               │ 7. Record Metrics             │              │               │
  │               │──┐           │               │              │               │
  │               │<─┘           │               │              │               │
  │               │              │               │              │               │
  │  Response     │              │               │              │               │
  │<──────────────┤              │               │              │               │
  │               │              │               │              │               │
```

## Cache Hit Flow (Fast Path)

```
Client          Server         Cache
  │               │              │
  │ POST /v1/chat/completions    │
  ├──────────────>│              │
  │               │              │
  │               │ 1. Lookup    │
  │               ├─────────────>│
  │               │              │
  │               │    L1 Check  │
  │               │<─────────────┤
  │               │    HIT ✓     │
  │               │              │
  │  Response     │              │
  │  (cached)     │              │
  │  <1ms         │              │
  │<──────────────┤              │
  │               │              │

Total Latency: <1ms
Cost: $0.00 (cached)
```

## Data Transformation Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                     Request Transformation                       │
└─────────────────────────────────────────────────────────────────┘

HTTP JSON                    Cache Format              Provider Format
    │                            │                          │
    ▼                            ▼                          ▼
┌──────────────┐    ┌───────────────────┐    ┌──────────────────────┐
│ ChatCompletion│    │ CacheableRequest  │    │  UnifiedRequest      │
│ Request       │    ├───────────────────┤    ├──────────────────────┤
├──────────────┤    │ model: String     │    │ model: String        │
│ model        │───>│ prompt: String    │───>│ messages: Vec<Msg>   │
│ messages[]   │    │ temperature: f32  │    │ temperature: f32     │
│ temperature  │    │ max_tokens: u32   │    │ max_tokens: usize    │
│ max_tokens   │    │ parameters: Map   │    │ stream: bool         │
│ stream       │    └───────────────────┘    │ metadata: Map        │
└──────────────┘              │               └──────────────────────┘
                              │                         │
                              ▼                         ▼
                    ┌───────────────────┐    ┌──────────────────────┐
                    │  SHA-256 Hash     │    │  OpenAI Format       │
                    │  Cache Key        │    │       OR             │
                    └───────────────────┘    │  Anthropic Format    │
                                             └──────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    Response Transformation                       │
└─────────────────────────────────────────────────────────────────┘

Provider Format              Unified Format            HTTP JSON
    │                            │                          │
    ▼                            ▼                          ▼
┌──────────────────┐   ┌────────────────────┐   ┌────────────────────┐
│ OpenAI Response  │   │ UnifiedResponse    │   │ ChatCompletion     │
│      OR          │   ├────────────────────┤   │ Response           │
│ Anthropic Resp.  │──>│ id: String         │──>├────────────────────┤
└──────────────────┘   │ model: String      │   │ id                 │
                       │ choices: Vec       │   │ object             │
                       │ usage: Usage       │   │ created            │
                       │ metadata           │   │ model              │
                       └────────────────────┘   │ choices[]          │
                                 │              │ usage              │
                                 ▼              │ metadata           │
                       ┌────────────────────┐   └────────────────────┘
                       │ CachedResponse     │
                       ├────────────────────┤
                       │ content: String    │
                       │ tokens: TokenUsage │
                       │ model: String      │
                       │ cached_at: i64     │
                       └────────────────────┘
```

## Component Dependencies

```
┌──────────────────────────────────────────────────────────────────┐
│                    llm-edge-agent (main)                         │
│                                                                   │
│  ├─ main.rs          (entry point, server setup)                │
│  ├─ lib.rs           (module exports)                            │
│  ├─ integration.rs   (app state, initialization)                │
│  └─ proxy.rs         (request handler, orchestration)           │
└──────────────────────────────────────────────────────────────────┘
                               │
                ┌──────────────┼──────────────┐
                │              │              │
                ▼              ▼              ▼
    ┌───────────────┐ ┌──────────────┐ ┌─────────────────┐
    │ llm-edge-cache│ │ llm-edge-    │ │ llm-edge-       │
    │               │ │ routing      │ │ providers       │
    ├───────────────┤ ├──────────────┤ ├─────────────────┤
    │ • L1 (Moka)   │ │ • Strategies │ │ • OpenAI        │
    │ • L2 (Redis)  │ │ • Circuit    │ │ • Anthropic     │
    │ • Keys        │ │   Breakers   │ │ • Adapter trait │
    │ • Metrics     │ │ • Fallback   │ │ • UnifiedReq/Res│
    └───────────────┘ └──────────────┘ └─────────────────┘
                │              │              │
                └──────────────┼──────────────┘
                               │
                ┌──────────────┼──────────────┐
                ▼              ▼              ▼
    ┌───────────────┐ ┌──────────────┐ ┌─────────────────┐
    │ llm-edge-     │ │ llm-edge-    │ │ llm-edge-       │
    │ monitoring    │ │ security     │ │ proxy           │
    ├───────────────┤ ├──────────────┤ ├─────────────────┤
    │ • Prometheus  │ │ • Auth       │ │ • Server        │
    │ • OpenTelemetry│ • PII         │ │ • Middleware    │
    │ • Metrics     │ │ • Validation │ │ • TLS           │
    │ • Tracing     │ └──────────────┘ │ • Routes        │
    └───────────────┘                  └─────────────────┘
```

## State Management

```
┌─────────────────────────────────────────────────────────────────┐
│                        AppState (Arc)                            │
│                   (Shared across all requests)                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────────────────────────────────────────┐        │
│  │  cache_manager: Arc<CacheManager>                   │        │
│  │    ├─ l1: L1Cache (Moka)                            │        │
│  │    ├─ l2: Option<L2Cache> (Redis)                   │        │
│  │    └─ metrics: CacheMetrics                         │        │
│  └─────────────────────────────────────────────────────┘        │
│                                                                   │
│  ┌─────────────────────────────────────────────────────┐        │
│  │  openai_provider: Option<Arc<dyn LLMProvider>>      │        │
│  │    └─ OpenAIAdapter { client, api_key, base_url }  │        │
│  └─────────────────────────────────────────────────────┘        │
│                                                                   │
│  ┌─────────────────────────────────────────────────────┐        │
│  │  anthropic_provider: Option<Arc<dyn LLMProvider>>   │        │
│  │    └─ AnthropicAdapter { client, api_key, ... }    │        │
│  └─────────────────────────────────────────────────────┘        │
│                                                                   │
│  ┌─────────────────────────────────────────────────────┐        │
│  │  config: Arc<AppConfig>                             │        │
│  │    ├─ host, port                                    │        │
│  │    ├─ enable_l2_cache, redis_url                   │        │
│  │    ├─ openai_api_key, anthropic_api_key            │        │
│  │    └─ enable_tracing, enable_metrics                │        │
│  └─────────────────────────────────────────────────────┘        │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
                            │
                            │ Cloned for each request
                            │ (cheap - Arc)
                            ▼
                    ┌───────────────┐
                    │ HTTP Handler  │
                    │ (async fn)    │
                    └───────────────┘
```

## Error Flow

```
┌────────────────────────────────────────────────────────────────┐
│                      Error Handling Flow                        │
└────────────────────────────────────────────────────────────────┘

Request
  │
  ├─> Validation Error ──────> ProxyError::ValidationError
  │                                    │
  │                                    ├─> 400 Bad Request
  │                                    └─> Log warning
  │
  ├─> Cache Error (L2) ──────> Log warning only
  │                              │
  │                              └─> Continue without cache
  │
  ├─> Provider Error ────────> ProxyError::ProviderError
  │                                    │
  │                                    ├─> 502 Bad Gateway
  │                                    ├─> Log error
  │                                    ├─> Record metric
  │                                    └─> Try fallback provider
  │
  └─> Internal Error ────────> ProxyError::InternalError
                                       │
                                       ├─> 500 Internal Server Error
                                       └─> Log error
```

## Observability Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                    Observability Stack                          │
└────────────────────────────────────────────────────────────────┘

Request Flow                    Observability Layer
     │                                  │
     │                      ┌───────────┼───────────┐
     │                      │           │           │
     ▼                      ▼           ▼           ▼
┌──────────┐        ┌───────────┐ ┌──────────┐ ┌──────────┐
│ Validate │───────>│ Metrics   │ │ Tracing  │ │ Logging  │
└──────────┘        ├───────────┤ ├──────────┤ ├──────────┤
     │              │ Prometheus│ │ OpenTel  │ │ Structured│
     ▼              │           │ │          │ │ (JSON)   │
┌──────────┐        │ record_*()│ │ #[inst-  │ │ info!()  │
│ Cache    │───────>│           │ │  rument] │ │ debug!() │
└──────────┘        │ counter!()│ │          │ │ warn!()  │
     │              │ histogram!│ │ Spans    │ │ error!() │
     ▼              │ gauge!()  │ │          │ │          │
┌──────────┐        └───────────┘ └──────────┘ └──────────┘
│ Route    │               │             │           │
└──────────┘               │             │           │
     │                     ▼             ▼           ▼
     ▼              ┌───────────┐ ┌──────────┐ ┌──────────┐
┌──────────┐        │/metrics   │ │ Jaeger   │ │ stdout   │
│ Provider │───────>│endpoint   │ │ (future) │ │ stderr   │
└──────────┘        │           │ │          │ │          │
     │              │ Scrape by │ │ Trace    │ │ Log      │
     ▼              │ Prometheus│ │ collector│ │ aggregator│
┌──────────┐        └───────────┘ └──────────┘ └──────────┘
│ Response │
└──────────┘

Metrics Collected:
- llm_edge_requests_total{provider, model, status}
- llm_edge_request_duration_ms{provider, model}
- llm_edge_cache_hits_total{tier}
- llm_edge_cache_misses_total{tier}
- llm_edge_tokens_total{provider, model, type}
- llm_edge_cost_usd_total{provider, model}
- llm_edge_provider_available{provider}
- llm_edge_active_requests
```

## Deployment Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                    Production Deployment                        │
└────────────────────────────────────────────────────────────────┘

                    ┌─────────────────┐
                    │  Load Balancer  │
                    │   (nginx/ALB)   │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
    ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
    │ LLM Edge     │ │ LLM Edge     │ │ LLM Edge     │
    │ Agent Pod 1  │ │ Agent Pod 2  │ │ Agent Pod 3  │
    ├──────────────┤ ├──────────────┤ ├──────────────┤
    │ L1: Moka     │ │ L1: Moka     │ │ L1: Moka     │
    │ (per-pod)    │ │ (per-pod)    │ │ (per-pod)    │
    └──────┬───────┘ └──────┬───────┘ └──────┬───────┘
           │                │                │
           └────────────────┼────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │  Redis Cluster        │
                │  (L2 Cache - Shared)  │
                └───────────────────────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
              ▼             ▼             ▼
    ┌──────────────┐ ┌────────────┐ ┌────────────┐
    │  Prometheus  │ │  Grafana   │ │   Jaeger   │
    │  (Metrics)   │ │ (Dashboards│ │  (Traces)  │
    └──────────────┘ └────────────┘ └────────────┘

Provider APIs:
- api.openai.com
- api.anthropic.com
- generativelanguage.googleapis.com
```

## Summary

This integration architecture provides:

✅ **Complete end-to-end flow** - From HTTP request to provider response
✅ **Multi-tier caching** - L1 (Moka) + L2 (Redis) for optimal performance
✅ **Provider abstraction** - Unified interface for all LLM providers
✅ **Intelligent routing** - Model-based with circuit breakers
✅ **Comprehensive observability** - Metrics, tracing, and logging
✅ **Production-ready** - Health checks, error handling, graceful degradation
✅ **Horizontal scalability** - Stateless design with shared L2 cache
✅ **Performance optimized** - <20ms overhead target

The system is designed for:
- High throughput (>1000 req/s)
- Low latency (<1ms cache hits)
- Cost optimization (caching reduces provider costs)
- Reliability (circuit breakers, fallbacks)
- Observability (metrics, traces, logs)
