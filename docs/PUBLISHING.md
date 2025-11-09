# Publishing Guide for LLM Edge Agent

This guide provides comprehensive instructions for publishing the LLM Edge Agent crates to crates.io and preparing for distribution.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Pre-Publishing Checklist](#pre-publishing-checklist)
- [Publishing Order](#publishing-order)
- [Step-by-Step Publishing Instructions](#step-by-step-publishing-instructions)
- [Post-Publishing Tasks](#post-publishing-tasks)
- [Version Management](#version-management)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### 1. Crates.io Account Setup

1. Create an account at [crates.io](https://crates.io/)
2. Generate an API token:
   ```bash
   cargo login
   ```
   This will prompt you to visit https://crates.io/me and copy your API token

### 2. Repository Requirements

- ✅ Repository must be public on GitHub
- ✅ Repository URL: `https://github.com/globalbusinessadvisors/llm-edge-agent`
- ✅ LICENSE file exists in repository root (Apache-2.0)
- ✅ All code is committed to git (or use `--allow-dirty` flag)

### 3. Build Requirements

- Rust 1.75 or later
- All dependencies available
- Tests passing

## Pre-Publishing Checklist

Before publishing, verify the following:

### Critical Requirements (MUST HAVE)

- [x] Repository URL updated in `Cargo.toml` (workspace level)
- [x] All crates have `description` field
- [x] All crates have `license` field (Apache-2.0)
- [x] All crates have `keywords` (5 max per crate)
- [x] All crates have `categories` (5 max per crate)
- [x] README.md exists for each crate
- [x] All tests pass: `cargo test --all`
- [ ] All changes committed to git (or use `--allow-dirty`)

### Recommended (SHOULD HAVE)

- [x] READMEs include installation instructions
- [x] READMEs include usage examples
- [x] READMEs include badges (crates.io, docs.rs, license)
- [ ] Documentation comments for all public APIs
- [ ] Examples directory with usage examples
- [ ] CHANGELOG.md for each crate

## Publishing Order

Due to internal dependencies, crates **MUST** be published in this order:

### Phase 1: Independent Crates (No Internal Dependencies)

These can be published in any order:

1. **llm-edge-providers** - Provider adapters (no deps)
2. **llm-edge-cache** - Caching system (no deps)
3. **llm-edge-monitoring** - Observability (no deps)
4. **llm-edge-security** - Security layer (no deps)
5. **llm-edge-proxy** - HTTP proxy (no deps)

### Phase 2: Dependent Crates

Must be published **AFTER** Phase 1:

6. **llm-edge-routing** - Depends on: `llm-edge-providers`
7. **llm-edge-agent** - Depends on: ALL crates above

## Step-by-Step Publishing Instructions

### Phase 1: Publish Independent Crates

#### 1. Publish llm-edge-providers

```bash
# Test the package
cargo publish --dry-run -p llm-edge-providers

# If successful, publish for real
cargo publish -p llm-edge-providers
```

Wait for the crate to be indexed on crates.io (~1-2 minutes)

#### 2. Publish llm-edge-cache

```bash
cargo publish --dry-run -p llm-edge-cache
cargo publish -p llm-edge-cache
```

#### 3. Publish llm-edge-monitoring

```bash
cargo publish --dry-run -p llm-edge-monitoring
cargo publish -p llm-edge-monitoring
```

#### 4. Publish llm-edge-security

```bash
cargo publish --dry-run -p llm-edge-security
cargo publish -p llm-edge-security
```

#### 5. Publish llm-edge-proxy

```bash
cargo publish --dry-run -p llm-edge-proxy
cargo publish -p llm-edge-proxy
```

**IMPORTANT:** Wait ~5 minutes for all Phase 1 crates to be fully indexed on crates.io before proceeding to Phase 2.

### Phase 2: Publish Dependent Crates

#### 6. Publish llm-edge-routing

```bash
# This will now work because llm-edge-providers is on crates.io
cargo publish --dry-run -p llm-edge-routing
cargo publish -p llm-edge-routing
```

#### 7. Publish llm-edge-agent

```bash
# This requires ALL previous crates to be published
cargo publish --dry-run -p llm-edge-agent
cargo publish -p llm-edge-agent
```

## Post-Publishing Tasks

### 1. Verify Publication

Check each crate on crates.io:
- https://crates.io/crates/llm-edge-providers
- https://crates.io/crates/llm-edge-cache
- https://crates.io/crates/llm-edge-monitoring
- https://crates.io/crates/llm-edge-security
- https://crates.io/crates/llm-edge-proxy
- https://crates.io/crates/llm-edge-routing
- https://crates.io/crates/llm-edge-agent

### 2. Verify Documentation

Check docs.rs build status:
- https://docs.rs/llm-edge-providers
- https://docs.rs/llm-edge-cache
- https://docs.rs/llm-edge-monitoring
- https://docs.rs/llm-edge-security
- https://docs.rs/llm-edge-proxy
- https://docs.rs/llm-edge-routing
- https://docs.rs/llm-edge-agent

Docs.rs automatically builds documentation for all published crates.

### 3. Update Root README

Add crates.io badges to the root README.md:

```markdown
[![Crates.io](https://img.shields.io/crates/v/llm-edge-agent.svg)](https://crates.io/crates/llm-edge-agent)
[![Documentation](https://docs.rs/llm-edge-agent/badge.svg)](https://docs.rs/llm-edge-agent)
```

### 4. Create GitHub Release

Tag the release:

```bash
git tag -a v0.1.0 -m "Release v0.1.0 - Initial crates.io publication"
git push origin v0.1.0
```

Create a GitHub release with:
- Release notes
- Installation instructions
- Links to published crates
- Links to documentation

### 5. Announce

Consider announcing on:
- Project blog/website
- Twitter/X
- Reddit r/rust
- This Week in Rust
- Rust Users Forum

## Version Management

### Versioning Strategy

All workspace crates use synchronized versioning:
- Current version: `0.1.0`
- Follows [Semantic Versioning](https://semver.org/)
- Keep all crates at the same version for simplicity

### Publishing Updates

When publishing updates:

1. Update version in `Cargo.toml` (workspace level):
   ```toml
   [workspace.package]
   version = "0.1.1"  # or 0.2.0, 1.0.0, etc.
   ```

2. Update all internal version references in crate `Cargo.toml` files:
   ```toml
   llm-edge-providers = { version = "0.1.1", path = "../llm-edge-providers" }
   ```

3. Update CHANGELOG.md files

4. Publish in the same order as initial release

### Version Yanking

If you need to yank a version:

```bash
cargo yank --vers 0.1.0 llm-edge-providers
```

To un-yank:

```bash
cargo yank --undo --vers 0.1.0 llm-edge-providers
```

## Troubleshooting

### Common Issues

#### "no matching package found"

**Problem:** Publishing a crate that depends on another workspace crate that hasn't been published yet.

**Solution:** Publish dependencies first (follow the publishing order).

#### "files in working directory contain changes"

**Problem:** Uncommitted changes in git.

**Solution:** Either commit changes or use `--allow-dirty`:
```bash
cargo publish --allow-dirty -p crate-name
```

#### "rate limit exceeded"

**Problem:** Publishing too many crates too quickly.

**Solution:** Wait 1-2 minutes between publishes.

#### "invalid category"

**Problem:** Category doesn't exist on crates.io.

**Solution:** Check valid categories at https://crates.io/categories

#### "crate name already exists"

**Problem:** Crate name is already taken.

**Solution:** Choose a different name. Current names are available:
- `llm-edge-agent`
- `llm-edge-proxy`
- `llm-edge-cache`
- `llm-edge-routing`
- `llm-edge-providers`
- `llm-edge-security`
- `llm-edge-monitoring`

### Documentation Build Failures

If docs.rs fails to build:

1. Check build logs at https://docs.rs/crate-name/version
2. Common causes:
   - Missing feature flags
   - Platform-specific code
   - External dependencies not available

3. Fix by adding to `Cargo.toml`:
   ```toml
   [package.metadata.docs.rs]
   all-features = true
   rustdoc-args = ["--cfg", "docsrs"]
   ```

## Dry Run Summary

All standalone crates have been tested and are ready to publish:

```
✅ llm-edge-providers - Ready to publish
✅ llm-edge-cache - Ready to publish
✅ llm-edge-monitoring - Ready to publish
✅ llm-edge-security - Ready to publish
✅ llm-edge-proxy - Ready to publish
⏸️  llm-edge-routing - Ready after llm-edge-providers is published
⏸️  llm-edge-agent - Ready after all dependencies are published
```

## Quick Publish Script

For automated publishing (use with caution):

```bash
#!/bin/bash
set -e

echo "Publishing Phase 1: Independent crates..."
cargo publish -p llm-edge-providers
sleep 30
cargo publish -p llm-edge-cache
sleep 30
cargo publish -p llm-edge-monitoring
sleep 30
cargo publish -p llm-edge-security
sleep 30
cargo publish -p llm-edge-proxy
sleep 30

echo "Waiting for crates.io indexing..."
sleep 180  # Wait 3 minutes

echo "Publishing Phase 2: Dependent crates..."
cargo publish -p llm-edge-routing
sleep 30
cargo publish -p llm-edge-agent

echo "All crates published successfully!"
```

**WARNING:** This script will publish for real. Test with `--dry-run` first!

## Additional Resources

- [Cargo Book - Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Crates.io Policies](https://crates.io/policies)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [docs.rs Documentation](https://docs.rs/about)

## Support

For questions or issues:
- Open an issue: https://github.com/globalbusinessadvisors/llm-edge-agent/issues
- Email: support@globalbusinessadvisors.com (update with actual contact)

---

**Last Updated:** 2025-11-08
**Version:** 0.1.0
**Prepared by:** Claude Code with LLM Edge Agent Team
