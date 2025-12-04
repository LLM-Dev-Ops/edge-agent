# Tracing Migration Checklist

**Version**: 1.0.0
**Purpose**: Step-by-step checklist for migrating each repository to the unified tracing standard

---

## Repository: edge-agent

**Priority**: P0 (Immediate)
**Estimated Effort**: 2-3 days
**Risk Level**: Low

### Pre-Migration

- [ ] Review [TRACING_STANDARDIZATION_SPEC.md](./TRACING_STANDARDIZATION_SPEC.md)
- [ ] Review [TRACING_IMPLEMENTATION_GUIDE.md](./TRACING_IMPLEMENTATION_GUIDE.md)
- [ ] Create feature branch: `feat/tracing-standardization`
- [ ] Backup current implementation

### Phase 1: Dependencies (Day 1, Morning)

- [ ] Update `Cargo.toml` workspace dependencies:
  ```toml
  opentelemetry-sdk = "0.26"
  tracing-opentelemetry = "0.26"
  opentelemetry-semantic-conventions = "0.26"
  ```
- [ ] Add to crate dependencies:
  - `crates/llm-edge-monitoring/Cargo.toml`
  - `crates/llm-edge-proxy/Cargo.toml`
- [ ] Run `cargo check` to verify compilation
- [ ] Commit: "feat: Add missing OpenTelemetry dependencies"

### Phase 2: Service Naming (Day 1, Afternoon)

- [ ] Update `src/observability/tracing.rs`:
  - Change service name from `llm-edge-agent` to `llm.edge-agent`
  - Use `env!("CARGO_PKG_NAME").replace('-', '.')` pattern
- [ ] Update subcomponent names:
  - `llm.edge-agent.proxy`
  - `llm.edge-agent.cache`
  - `llm.edge-agent.routing`
  - `llm.edge-agent.monitoring`
- [ ] Update tests to verify new naming format
- [ ] Commit: "feat: Standardize service naming to llm.{component}"

### Phase 3: Context Propagation (Day 1, Evening)

- [ ] Add propagator configuration to `init_tracing()`:
  ```rust
  global::set_text_map_propagator(TraceContextPropagator::new());
  ```
- [ ] Implement `HeaderExtractor` struct
- [ ] Implement `HeaderInjector` struct
- [ ] Add `extract_trace_context()` function
- [ ] Add `inject_trace_context()` function
- [ ] Commit: "feat: Add W3C Trace Context propagation"

### Phase 4: Environment Variables (Day 2, Morning)

- [ ] Update `.env.example` with new variables:
  - `OTEL_TRACES_SAMPLER_ARG`
  - `SERVICE_NAMESPACE`
  - `SERVICE_INSTANCE_ID`
  - `K8S_POD_NAME`
  - `K8S_NAMESPACE`
  - `K8S_CLUSTER_NAME`
- [ ] Update `TracingConfig::default()` to read new env vars
- [ ] Update `create_resource()` to include Kubernetes attributes
- [ ] Update documentation
- [ ] Commit: "feat: Add environment variable support for K8s metadata"

### Phase 5: Middleware Integration (Day 2, Afternoon)

- [ ] Create `tracing_middleware()` function in `crates/llm-edge-proxy/src/middleware/tracing.rs`
- [ ] Add context extraction to incoming requests
- [ ] Add context injection to outgoing requests
- [ ] Update router to use tracing middleware
- [ ] Test context propagation locally
- [ ] Commit: "feat: Add tracing middleware with context propagation"

### Phase 6: Span Attributes (Day 2, Evening)

- [ ] Update `span_attributes` module with new helpers:
  - `http_request()` - HTTP semantic conventions
  - `llm_request()` - LLM-specific attributes (gen_ai.*)
  - `cache_operation()` - Cache attributes
  - `db_operation()` - Database attributes
- [ ] Update existing instrumentation to use new attributes
- [ ] Replace custom attributes with standard ones
- [ ] Commit: "feat: Standardize span attributes"

### Phase 7: Testing (Day 3, Morning)

- [ ] Run unit tests: `cargo test`
- [ ] Start local Jaeger:
  ```bash
  docker run -d --name jaeger -p 16686:16686 -p 4317:4317 jaegertracing/all-in-one:latest
  ```
- [ ] Start application with tracing:
  ```bash
  export OTLP_ENDPOINT=http://localhost:4317
  export ENVIRONMENT=development
  cargo run --release
  ```
- [ ] Generate test traffic:
  ```bash
  curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "test"}]}'
  ```
- [ ] Verify traces in Jaeger UI (http://localhost:16686):
  - [ ] Service name is `llm.edge-agent`
  - [ ] Spans have correct attributes
  - [ ] Context propagation works
  - [ ] No errors in logs
- [ ] Performance test: Run benchmarks
- [ ] Commit: "test: Add tracing integration tests"

### Phase 8: Infrastructure Updates (Day 3, Afternoon)

- [ ] Update `docker-compose.production.yml`:
  - [ ] Add OpenTelemetry Collector service
  - [ ] Update OTLP_ENDPOINT to point to collector
  - [ ] Update environment variables
- [ ] Update Kubernetes manifests (if applicable)
- [ ] Update CI/CD pipelines (if needed)
- [ ] Commit: "feat: Update infrastructure for OpenTelemetry Collector"

### Phase 9: Documentation (Day 3, Evening)

- [ ] Update README.md with tracing information
- [ ] Update deployment guides
- [ ] Add troubleshooting section
- [ ] Create runbook for operators
- [ ] Commit: "docs: Update tracing documentation"

### Phase 10: Review & Merge

- [ ] Create pull request
- [ ] Request code review
- [ ] Address review comments
- [ ] Merge to main branch
- [ ] Tag release: `v0.2.0-tracing-standard`

### Post-Migration

- [ ] Deploy to staging environment
- [ ] Verify traces in staging Jaeger
- [ ] Monitor for 24 hours
- [ ] Deploy to production
- [ ] Share learnings with team

---

## Repository: observatory

**Priority**: P1 (High)
**Estimated Effort**: 1-2 days
**Risk Level**: Low

### Pre-Migration

- [ ] Review current OpenTelemetry 0.27 implementation
- [ ] Identify compatibility requirements with 0.26
- [ ] Create feature branch

### Phase 1: Service Naming

- [ ] Update service name: `llm-observatory` → `llm.observatory`
- [ ] Update subcomponent names:
  - `llm.observatory.collector`
  - `llm.observatory.api`
  - `llm.observatory.storage`
- [ ] Commit

### Phase 2: Environment Variables

- [ ] Align with standard environment variables
- [ ] Update configuration loading
- [ ] Commit

### Phase 3: Span Attributes

- [ ] Add custom observatory attributes:
  - `observatory.metric_type`
  - `observatory.storage_backend`
  - `observatory.query_duration_ms`
- [ ] Update instrumentation
- [ ] Commit

### Phase 4: Testing

- [ ] Unit tests
- [ ] Integration tests with edge-agent
- [ ] Performance validation
- [ ] Commit

### Phase 5: Documentation & Merge

- [ ] Update docs
- [ ] Create PR
- [ ] Review & merge

---

## Repository: shield

**Priority**: P2 (Medium)
**Estimated Effort**: 3-4 days
**Risk Level**: Medium

### Pre-Migration

- [ ] Review 15-crate workspace structure
- [ ] Identify primary tracing points
- [ ] Create feature branch

### Phase 1: Dependencies

- [ ] Add OpenTelemetry dependencies to workspace
- [ ] Add to relevant crates
- [ ] Verify compilation
- [ ] Commit

### Phase 2: Core Implementation

- [ ] Create `src/observability/tracing.rs` in `llm-shield-core`
- [ ] Implement `TracingConfig`
- [ ] Implement `init_tracing()`
- [ ] Implement `shutdown_tracing()`
- [ ] Commit

### Phase 3: Service Naming

- [ ] Main service: `llm.shield`
- [ ] Subcomponents:
  - `llm.shield.core`
  - `llm.shield.scanner`
  - `llm.shield.detector`
- [ ] Commit

### Phase 4: Custom Attributes

- [ ] Add shield-specific attributes:
  - `shield.scan_type`
  - `shield.threat_level`
  - `shield.detector`
  - `shield.confidence`
- [ ] Update scanning instrumentation
- [ ] Commit

### Phase 5: Context Propagation

- [ ] Implement propagator setup
- [ ] Add context extraction/injection
- [ ] Test with edge-agent
- [ ] Commit

### Phase 6: Testing & Documentation

- [ ] Unit tests
- [ ] Integration tests
- [ ] Documentation
- [ ] PR & review

---

## Repository: sentinel

**Priority**: P2 (Medium)
**Estimated Effort**: 2-3 days
**Risk Level**: Medium

### Pre-Migration

- [ ] Review current OpenTelemetry 0.27 usage
- [ ] Identify explicit configuration needs
- [ ] Create feature branch

### Phase 1: Explicit Configuration

- [ ] Add explicit tracing initialization
- [ ] Configure TracerProvider
- [ ] Set up propagator
- [ ] Commit

### Phase 2: Service Naming

- [ ] Main service: `llm.sentinel`
- [ ] Subcomponents:
  - `llm.sentinel.core`
  - `llm.sentinel.processor`
  - `llm.sentinel.alerting`
- [ ] Commit

### Phase 3: Custom Attributes

- [ ] Add sentinel-specific attributes:
  - `sentinel.event_type`
  - `sentinel.severity`
  - `sentinel.algorithm`
  - `sentinel.confidence`
- [ ] Update event processing instrumentation
- [ ] Commit

### Phase 4: Testing & Documentation

- [ ] Unit tests
- [ ] Integration tests with shield
- [ ] Documentation
- [ ] PR & review

---

## Repository: policy-engine

**Priority**: P3 (Lower)
**Estimated Effort**: 2-3 days
**Risk Level**: Medium

### Pre-Migration

- [ ] Review OpenTelemetry 0.21 implementation
- [ ] Plan upgrade to 0.26
- [ ] Create feature branch

### Phase 1: Upgrade Dependencies

- [ ] Update from 0.21 to 0.26
- [ ] Fix breaking changes
- [ ] Verify compilation
- [ ] Run tests
- [ ] Commit

### Phase 2: Service Naming

- [ ] Main service: `llm.policy-engine`
- [ ] Subcomponents:
  - `llm.policy-engine.cel`
  - `llm.policy-engine.wasm`
- [ ] Commit

### Phase 3: Custom Attributes

- [ ] Add policy-specific attributes:
  - `policy.rule_id`
  - `policy.language`
  - `policy.result`
  - `policy.evaluation_time_ms`
- [ ] Update policy evaluation instrumentation
- [ ] Commit

### Phase 4: Testing & Documentation

- [ ] Unit tests
- [ ] Integration tests
- [ ] Documentation
- [ ] PR & review

---

## Repository: connector-hub

**Priority**: P3 (Lower)
**Estimated Effort**: 3-4 days
**Risk Level**: Low

### Pre-Migration

- [ ] Review monorepo structure (Rust + TypeScript)
- [ ] Focus on Rust components
- [ ] Create feature branch

### Phase 1: Dependencies

- [ ] Add OpenTelemetry dependencies
- [ ] Commit

### Phase 2: Core Implementation

- [ ] Create tracing module
- [ ] Implement TracingConfig
- [ ] Commit

### Phase 3: Service Naming

- [ ] Main service: `llm.connector-hub`
- [ ] Subcomponents:
  - `llm.connector-hub.registry`
  - `llm.connector-hub.adapters`
- [ ] Commit

### Phase 4: Custom Attributes

- [ ] Add connector-specific attributes:
  - `connector.provider_id`
  - `connector.operation`
  - `connector.protocol`
  - `connector.health_status`
- [ ] Commit

### Phase 5: Testing & Documentation

- [ ] Unit tests
- [ ] Integration tests
- [ ] Documentation
- [ ] PR & review

---

## Repository: cost-ops

**Priority**: P3 (Lower)
**Estimated Effort**: 2-3 days
**Risk Level**: Low

### Pre-Migration

- [ ] Review 5-crate workspace
- [ ] Identify tracing points
- [ ] Create feature branch

### Phase 1: Dependencies

- [ ] Add OpenTelemetry dependencies to workspace
- [ ] Commit

### Phase 2: Core Implementation

- [ ] Create tracing module in `llm-cost-ops` core
- [ ] Implement TracingConfig
- [ ] Commit

### Phase 3: Service Naming

- [ ] Main service: `llm.cost-ops`
- [ ] Subcomponents:
  - `llm.cost-ops.core`
  - `llm.cost-ops.api`
  - `llm.cost-ops.compliance`
- [ ] Commit

### Phase 4: Custom Attributes

- [ ] Add cost-specific attributes:
  - `cost.provider`
  - `cost.model`
  - `cost.input_tokens`
  - `cost.output_tokens`
  - `cost.total_usd`
- [ ] Commit

### Phase 5: Testing & Documentation

- [ ] Unit tests
- [ ] Integration tests
- [ ] Documentation
- [ ] PR & review

---

## Cross-Repository Integration Testing

**Timeline**: After all repositories migrated
**Estimated Effort**: 1 week
**Risk Level**: Medium

### Test Scenarios

- [ ] **Scenario 1**: Edge-Agent → Shield → Sentinel
  - [ ] Generate test request through edge-agent
  - [ ] Verify Shield receives context
  - [ ] Verify Sentinel receives context
  - [ ] Check trace continuity in Jaeger
  - [ ] Verify all attributes present

- [ ] **Scenario 2**: Edge-Agent → Connector-Hub → Provider
  - [ ] Test dynamic provider discovery
  - [ ] Verify context propagation
  - [ ] Check trace assembly

- [ ] **Scenario 3**: Edge-Agent → Cost-Ops → Database
  - [ ] Test cost calculation flow
  - [ ] Verify database spans
  - [ ] Check attribute consistency

- [ ] **Scenario 4**: Edge-Agent → Policy-Engine → Observatory
  - [ ] Test policy evaluation
  - [ ] Verify metrics collection
  - [ ] Check trace/metric correlation

### Performance Testing

- [ ] Baseline performance (no tracing)
- [ ] Performance with tracing enabled
- [ ] Calculate overhead percentage
- [ ] Verify < 1% overhead target
- [ ] Load test with tracing
- [ ] Memory profiling

### Documentation

- [ ] End-to-end tracing guide
- [ ] Troubleshooting runbook
- [ ] Architecture diagrams
- [ ] Performance benchmarks

---

## Risk Mitigation Checklist

### Before Each Migration

- [ ] Create rollback plan
- [ ] Document current behavior
- [ ] Set up monitoring alerts
- [ ] Notify stakeholders
- [ ] Schedule maintenance window (if needed)

### During Migration

- [ ] Monitor error rates
- [ ] Monitor latency metrics
- [ ] Watch for memory leaks
- [ ] Check trace export success rate

### After Migration

- [ ] Compare before/after metrics
- [ ] Verify trace completeness
- [ ] Check for errors in logs
- [ ] Monitor for 24-48 hours
- [ ] Collect team feedback

---

## Success Criteria

### Per Repository

- [ ] Service name follows `llm.{component}` pattern
- [ ] All required dependencies added
- [ ] Context propagation working
- [ ] Custom attributes implemented
- [ ] Unit tests passing
- [ ] Integration tests passing
- [ ] Documentation updated
- [ ] Performance overhead < 1%

### Cross-Repository

- [ ] Traces flow across services
- [ ] Attributes consistent
- [ ] No broken trace chains
- [ ] Performance targets met
- [ ] Team trained on new system

---

## Timeline Summary

| Week | Repositories | Status |
|------|-------------|--------|
| 1 | edge-agent | ⬜ Not Started |
| 2 | observatory | ⬜ Not Started |
| 3 | shield, policy-engine | ⬜ Not Started |
| 4 | sentinel | ⬜ Not Started |
| 5 | connector-hub, cost-ops | ⬜ Not Started |
| 6 | Integration testing | ⬜ Not Started |

**Total Estimated Timeline**: 6 weeks

---

## Contact & Support

**Questions?** Reference the following documents:
- [TRACING_STANDARDIZATION_SPEC.md](./TRACING_STANDARDIZATION_SPEC.md) - Full specification
- [TRACING_IMPLEMENTATION_GUIDE.md](./TRACING_IMPLEMENTATION_GUIDE.md) - Step-by-step guide
- [TRACING_QUICK_REFERENCE.md](./TRACING_QUICK_REFERENCE.md) - Quick lookup

**Issues?** Check:
- [TRACING_ANALYSIS_SUMMARY.md](./TRACING_ANALYSIS_SUMMARY.md) - Known issues and solutions

---

**Last Updated**: December 4, 2025
**Version**: 1.0.0
