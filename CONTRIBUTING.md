# Contributing to LLM Edge Agent

Thank you for your interest in contributing to LLM Edge Agent! This document provides guidelines and instructions for contributing.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Process](#development-process)
4. [Pull Request Process](#pull-request-process)
5. [Coding Standards](#coding-standards)
6. [Testing Requirements](#testing-requirements)
7. [Documentation](#documentation)
8. [Release Process](#release-process)

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive environment for all contributors, regardless of experience level, background, or identity.

### Expected Behavior

- Be respectful and considerate
- Welcome newcomers and help them get started
- Provide constructive feedback
- Focus on what is best for the community
- Show empathy towards other community members

### Unacceptable Behavior

- Harassment, discrimination, or offensive comments
- Personal attacks or trolling
- Publishing others' private information
- Any conduct that could reasonably be considered inappropriate

## Getting Started

### Prerequisites

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/llm-edge-agent.git
   cd llm-edge-agent
   ```

3. Set up the upstream remote:
   ```bash
   git remote add upstream https://github.com/yourusername/llm-edge-agent.git
   ```

4. Install development tools:
   ```bash
   rustup component add rustfmt clippy
   cargo install cargo-watch cargo-tarpaulin cargo-audit
   ```

5. Set up environment:
   ```bash
   cp .env.example .env
   # Configure your .env file
   ```

### Finding Issues to Work On

- Look for issues labeled `good first issue` for newcomers
- Issues labeled `help wanted` are actively seeking contributors
- Check the project board for current priorities

## Development Process

### Creating a Feature Branch

```bash
# Update your main branch
git checkout main
git pull upstream main

# Create a feature branch
git checkout -b feature/your-feature-name
```

### Branch Naming Convention

- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates
- `refactor/description` - Code refactoring
- `test/description` - Test improvements

### Making Changes

1. **Write code following our standards** (see below)
2. **Add tests** for new functionality
3. **Update documentation** as needed
4. **Run tests locally**:
   ```bash
   cargo test
   cargo fmt --check
   cargo clippy -- -D warnings
   ```

### Commit Messages

Follow conventional commits format:

```
type(scope): brief description

Longer description if needed, explaining:
- Why this change is needed
- What problem it solves
- Any trade-offs or design decisions

Closes #123
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `refactor`: Code refactoring
- `test`: Test additions or changes
- `chore`: Build process or tooling changes
- `perf`: Performance improvements

Examples:
```
feat(cache): add L3 semantic cache support

Implements semantic caching using Qdrant vector database.
Includes similarity search with configurable threshold.

Closes #42

---

fix(routing): prevent panic on empty provider list

Added validation to ensure at least one provider is available
before attempting routing decision.

Fixes #56
```

## Pull Request Process

### Before Submitting

1. **Update your branch** with latest main:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run all checks**:
   ```bash
   cargo fmt
   cargo clippy --fix
   cargo test --all-features
   cargo audit
   ```

3. **Update documentation**:
   - Update README.md if adding user-facing features
   - Add/update doc comments for public APIs
   - Update CHANGELOG.md

### Submitting the PR

1. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Create pull request** on GitHub with:
   - Clear title describing the change
   - Description explaining what and why
   - Link to related issues
   - Screenshots/examples if applicable

3. **Fill out the PR template**:
   - [ ] Tests added/updated
   - [ ] Documentation updated
   - [ ] CHANGELOG.md updated
   - [ ] No breaking changes (or documented)
   - [ ] CI passes

### PR Review Process

- Maintainers will review within 48 hours
- Address feedback by pushing new commits
- Once approved, maintainers will merge
- PRs may be marked as "changes requested" - address feedback and request re-review

### PR Checklist

- [ ] Branch is up-to-date with main
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No linter warnings (`cargo clippy`)
- [ ] Security audit passes (`cargo audit`)
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated (for notable changes)
- [ ] Commit messages follow convention

## Coding Standards

### Rust Style Guide

Follow the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/):

- Use `rustfmt` with default settings
- Maximum line length: 100 characters
- Use meaningful variable names
- Prefer explicit types in public APIs

### Error Handling

```rust
// ✅ Good: Use Result and ? operator
pub fn process_request(req: Request) -> Result<Response, Error> {
    let validated = validate_request(&req)?;
    let result = send_to_provider(validated)?;
    Ok(result)
}

// ❌ Bad: Using unwrap() in production code
pub fn process_request(req: Request) -> Response {
    let validated = validate_request(&req).unwrap();
    send_to_provider(validated).unwrap()
}
```

### Async Code

```rust
// ✅ Good: Use async/await
pub async fn fetch_data(url: &str) -> Result<Data> {
    let response = client.get(url).await?;
    let data = response.json().await?;
    Ok(data)
}

// ❌ Bad: Blocking in async context
pub async fn fetch_data(url: &str) -> Result<Data> {
    let response = reqwest::blocking::get(url)?;  // Blocks the executor!
    Ok(response.json()?)
}
```

### Documentation

Add doc comments for all public items:

```rust
/// Manages multi-tier caching for LLM responses.
///
/// The cache manager coordinates between L1 (in-memory), L2 (Redis),
/// and L3 (semantic) cache tiers, falling back gracefully when tiers
/// are unavailable.
///
/// # Examples
///
/// ```
/// let cache = CacheManager::new_l1_only(1000);
/// cache.put(key, value).await;
/// ```
pub struct CacheManager {
    // ...
}
```

### Security

- Never log API keys or secrets
- Use `secrecy::Secret` for sensitive data
- Validate all user input
- Use parameterized queries (future SQL support)
- Keep dependencies updated

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key1 = CacheKey::from_prompt("hello", "gpt-4", None);
        let key2 = CacheKey::from_prompt("hello", "gpt-4", None);
        assert_eq!(key1, key2);
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = some_async_function().await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Place in `tests/` directory or mark with `#[ignore]`:

```rust
#[tokio::test]
#[ignore]  // Requires Redis running
async fn test_redis_integration() {
    let cache = L2Cache::new("redis://localhost:6379").await.unwrap();
    // Test Redis operations
}
```

Run with: `cargo test -- --ignored`

### Test Coverage

Aim for:
- MVP: >70% coverage
- Beta: >80% coverage
- v1.0: >85% coverage

Check coverage:
```bash
cargo tarpaulin --verbose --all-features --workspace
```

### Performance Tests

Add benchmarks in `benches/`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_cache_lookup(c: &mut Criterion) {
    c.bench_function("l1_cache_get", |b| {
        let cache = L1Cache::new(1000);
        b.iter(|| {
            cache.get(black_box(&key))
        });
    });
}

criterion_group!(benches, benchmark_cache_lookup);
criterion_main!(benches);
```

## Documentation

### Types of Documentation

1. **Code Comments**: Explain why, not what
2. **Doc Comments**: Public API documentation
3. **README**: Project overview and quick start
4. **Guides**: QUICKSTART.md, DEVELOPMENT.md
5. **Architecture**: plans/ directory

### When to Update Docs

- Adding public APIs
- Changing behavior
- Adding configuration options
- Modifying deployment process
- Discovering gotchas or best practices

## Release Process

Releases follow semantic versioning (MAJOR.MINOR.PATCH):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes

### Release Checklist

1. Update CHANGELOG.md
2. Update version in Cargo.toml
3. Create release tag: `git tag -a v0.1.0 -m "Release v0.1.0"`
4. Push tag: `git push origin v0.1.0`
5. GitHub Actions will build and publish release

## Questions?

- **General questions**: Open a GitHub Discussion
- **Bug reports**: Open a GitHub Issue
- **Security issues**: Email security@example.com (do NOT open public issue)

## Recognition

Contributors will be:
- Listed in CHANGELOG.md
- Credited in release notes
- Added to CONTRIBUTORS.md

Thank you for contributing to LLM Edge Agent!
