# Benchmark Summary

This file contains the most recent benchmark results for LLM Edge Agent.

## Last Run

**Date:** Not yet run
**Status:** Pending first benchmark execution

## Results

No benchmark results available yet. Run benchmarks using:

```bash
npm run bench
# or
llm-edge-agent benchmark run
```

## Benchmark Categories

### Cache Performance
- L1 cache operations (write, read hit, read miss)
- Cache size scaling (100, 1000, 10000 entries)
- Cache key generation
- Concurrent access patterns

### Routing Performance
- Model-based routing
- Cost-optimized routing
- Latency-optimized routing
- Failover routing
- Routing scalability with varying provider counts

## Output Files

Raw benchmark data is stored in `benchmarks/output/raw/` with timestamps:
- `cache-YYYYMMDD-HHMMSS.json` - Cache benchmark results
- `routing-YYYYMMDD-HHMMSS.json` - Routing benchmark results

This summary file is automatically updated after each benchmark run.
