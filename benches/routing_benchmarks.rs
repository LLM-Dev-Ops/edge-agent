// Routing Performance Benchmarks
// Measures routing decision performance for different strategies

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use llm_edge_routing::{Router, RoutingStrategy, RoutingRequest, ProviderHealth};

fn create_test_request(model: &str) -> RoutingRequest {
    RoutingRequest {
        model: model.to_string(),
        estimated_tokens: 100,
        priority: 1,
    }
}

fn create_providers() -> Vec<ProviderHealth> {
    vec![
        ProviderHealth {
            name: "openai".to_string(),
            healthy: true,
            latency_ms: 150,
            error_rate: 0.01,
            cost_per_1k_tokens: 0.002,
        },
        ProviderHealth {
            name: "anthropic".to_string(),
            healthy: true,
            latency_ms: 180,
            error_rate: 0.005,
            cost_per_1k_tokens: 0.003,
        },
        ProviderHealth {
            name: "cohere".to_string(),
            healthy: true,
            latency_ms: 200,
            error_rate: 0.02,
            cost_per_1k_tokens: 0.001,
        },
    ]
}

// Benchmark: Model-based routing (simple)
fn bench_model_based_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("routing_model_based");

    let router = Router::new(RoutingStrategy::ModelBased);
    let providers = create_providers();

    let models = vec!["gpt-4", "gpt-3.5-turbo", "claude-3-opus", "claude-3-sonnet"];

    for model in models {
        group.bench_with_input(BenchmarkId::from_parameter(model), &model, |b, &model| {
            b.iter(|| {
                let request = create_test_request(model);
                black_box(router.route(&request, &providers))
            });
        });
    }

    group.finish();
}

// Benchmark: Cost-optimized routing
fn bench_cost_optimized_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("routing_cost_optimized");

    let router = Router::new(RoutingStrategy::CostOptimized);
    let providers = create_providers();

    group.bench_function("route_decision", |b| {
        b.iter(|| {
            let request = create_test_request("gpt-3.5-turbo");
            black_box(router.route(&request, &providers))
        });
    });

    group.finish();
}

// Benchmark: Latency-optimized routing
fn bench_latency_optimized_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("routing_latency_optimized");

    let router = Router::new(RoutingStrategy::LatencyOptimized);
    let providers = create_providers();

    group.bench_function("route_decision", |b| {
        b.iter(|| {
            let request = create_test_request("gpt-3.5-turbo");
            black_box(router.route(&request, &providers))
        });
    });

    group.finish();
}

// Benchmark: Failover routing with unhealthy providers
fn bench_failover_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("routing_failover");

    let router = Router::new(RoutingStrategy::Failover);

    // Mix of healthy and unhealthy providers
    let mut providers = create_providers();
    providers[0].healthy = false;  // Mark first provider as unhealthy

    group.bench_function("route_with_failover", |b| {
        b.iter(|| {
            let request = create_test_request("gpt-3.5-turbo");
            black_box(router.route(&request, &providers))
        });
    });

    group.finish();
}

// Benchmark: Routing with varying number of providers
fn bench_routing_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("routing_scalability");

    let router = Router::new(RoutingStrategy::CostOptimized);

    for num_providers in [5, 10, 20, 50].iter() {
        let mut providers = vec![];
        for i in 0..*num_providers {
            providers.push(ProviderHealth {
                name: format!("provider_{}", i),
                healthy: true,
                latency_ms: 100 + (i as u64 * 10),
                error_rate: 0.01,
                cost_per_1k_tokens: 0.001 + (i as f64 * 0.0001),
            });
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(num_providers),
            num_providers,
            |b, _| {
                b.iter(|| {
                    let request = create_test_request("gpt-3.5-turbo");
                    black_box(router.route(&request, &providers))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_model_based_routing,
    bench_cost_optimized_routing,
    bench_latency_optimized_routing,
    bench_failover_routing,
    bench_routing_scalability
);

criterion_main!(benches);
