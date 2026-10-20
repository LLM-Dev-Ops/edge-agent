# Native Cloud Function contract validation

The contract and native HTTP probe were frozen before source changes, against
parent `11da64cb7e409beac9b3c3647767715d6f0b7101`: 0/21 malformed/defaulting
holdouts, 12/12 valid-behavior anchors, 33/33 existing tests, npm audit zero.
Acceptance requires 21/21, all anchors, all original test names, no new runtime
dependencies or skips, and zero npm findings. Repository Rust/security/CI and
required approval gates remain independent requirements for merge.

The existing hand-written validator checked only top-level presence. It now
enforces exactly the schema subset used by the published contracts: local
definition references, nested object/array/scalar types, required own fields,
safe integers, finite numbers, enum and numeric minima. It does not coerce
strings, accept array objects, or let malformed guard limits reach arithmetic.
Unknown request properties stay permitted for forward compatibility. This is
not a general JSON Schema engine. Work is capped at 10,000 visits, depth32 and
32 diagnostics. Optional numeric fields may be omitted, but explicit null is
not accepted by a nonnullable published type.

Omitted current_concurrency defaults to zero consistently with the optional
contract. This is not measured distributed concurrency/rate: callers supply
estimates. The function does not authenticate those estimates or add counters,
storage, model calls, policy updates or upstream Rust orchestration. This patch
does not prove deployment IAM or Rust backend correctness.

The probe uses the installed Google Functions Framework's public `http()` and
`getTestServer()` interfaces, the actual entry point and native fetch to an
ephemeral loopback listener. It uses no request/response or policy mocks. The
existing 33 mock-request tests are retained unchanged as regression coverage.
The added suite has 37 leaf assertions (38 Node test entries including its
container), for 70 leaves in total; no skipped tests.

```sh
npm ci --ignore-scripts
npm test
npm run test:cli
npm audit
node security/probe-contract.cjs /absolute/path/to/checkout
npm ci --ignore-scripts --prefix security/tools
node security/evaluate-contract.mjs BASELINE_CHECKOUT CANDIDATE_CHECKOUT TOOLS_PREFIX OUTPUT_JSON
node security/benchmark-contract.cjs BASELINE_CHECKOUT CANDIDATE_CHECKOUT OUTPUT_JSON
```

Flywheel0.1.12 runs both revisions live, preserves the original assertions,
signs lineage and the primary contract's inverse bytes, retains the complete
two-file parent source map with a verified hash, and verifies replay with its frozen
gate. Only a missing HTTP response counts as a no-op; an incorrect response is
a failed case. Cost is unmeasured (neutral zero gate input). Five alternating
benchmark trials retain raw samples; no production speedup or savings is claimed.

For native Rust builds on the existing Ubuntu ruOS desktop, first run the
current research-lab scripts/bootstrap-ruos.sh and verify pkg-config OpenSSL.
This workspace additionally needs libclang-dev for zstd-sys/bindgen. Install
with `sudo apt-get install -y --no-install-recommends libclang-dev` when missing,
then run locked tests/build/Clippy and audit the committed Cargo.lock. Do not
regenerate the lockfile to hide findings or assume prior runtime repairs persist.

Rollback before merge: close the PR. After an authorized merge, revert the
contract-validation commit and rebuild/revalidate (or revert the PR merge).
The signed inverse and companion parent source map contain exact bytes/hashes; they
restores the original validation defects. No deployed rollback or data migration
is performed. CORS hardening from the earlier branch should remain in place.
