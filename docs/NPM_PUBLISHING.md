# NPM Publishing Guide

This guide explains how to publish `@llm-dev-ops/llm-edge-agent` to the npm registry.

## Overview

The LLM Edge Agent uses a **platform-specific binary distribution** strategy via npm's `optionalDependencies` feature. This is the same approach used by successful projects like `esbuild`, `swc`, and `prisma`.

### Package Structure

- **Main Package**: `@llm-dev-ops/llm-edge-agent` - CLI wrapper and installation scripts
- **Platform Packages**: Binary-only packages for each supported platform:
  - `@llm-dev-ops/llm-edge-agent-linux-x64`
  - `@llm-dev-ops/llm-edge-agent-linux-arm64`
  - `@llm-dev-ops/llm-edge-agent-darwin-x64`
  - `@llm-dev-ops/llm-edge-agent-darwin-arm64`
  - `@llm-dev-ops/llm-edge-agent-windows-x64`
  - `@llm-dev-ops/llm-edge-agent-windows-arm64`

## Prerequisites

### 1. NPM Account Setup

1. Create an npm account at https://www.npmjs.com/signup
2. Join the `@llm-dev-ops` organization (or create it)
3. Generate an NPM access token:
   ```bash
   npm login
   npm token create --read-write
   ```
4. Save the token securely - you'll need it for GitHub Actions

### 2. GitHub Repository Setup

1. Add `NPM_TOKEN` to repository secrets:
   - Go to: Repository → Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Name: `NPM_TOKEN`
   - Value: Your npm access token from step 1.3

### 3. Verify Permissions

```bash
# Login to npm
npm login

# Verify authentication
npm whoami

# Check organization access
npm org ls @llm-dev-ops
```

## Publishing Workflow

### Automated Publishing (Recommended)

The project includes a comprehensive GitHub Actions workflow for automated publishing.

#### Step 1: Prepare for Release

1. **Update version numbers** in both:
   - `package.json` (line 3)
   - `Cargo.toml` (line 3)

   They must match! The prepack script validates this.

   ```bash
   # Update to version 0.2.0 (example)
   npm version 0.2.0 --no-git-tag-version
   ```

   Then manually update `Cargo.toml`:
   ```toml
   version = "0.2.0"
   ```

2. **Update CHANGELOG.md**:
   ```markdown
   ## [0.2.0] - 2024-11-09

   ### Added
   - Enhanced CLI with commander.js
   - Health check command
   - Config init command

   ### Fixed
   - GitHub Actions workflow dependency
   - prepublishOnly script
   ```

3. **Commit changes**:
   ```bash
   git add package.json Cargo.toml CHANGELOG.md
   git commit -m "chore: bump version to 0.2.0"
   git push origin main
   ```

#### Step 2: Dry Run (Recommended First)

1. Go to GitHub: **Actions** → **Publish to npm** → **Run workflow**
2. Select options:
   - Branch: `main`
   - Publishing type: **dry-run**
   - Version: (leave empty to use package.json version)
3. Click **Run workflow**
4. Monitor the workflow execution
5. Review the dry-run output to ensure everything is correct

Expected output:
- ✅ All platform binaries build successfully
- ✅ All platform packages are created
- ✅ Validation passes
- ✅ No actual publishing occurs

#### Step 3: Publish Platform Packages

1. Go to GitHub: **Actions** → **Publish to npm** → **Run workflow**
2. Select options:
   - Branch: `main`
   - Publishing type: **platforms-only**
   - Version: (leave empty)
3. Click **Run workflow**
4. Wait for completion (typically 15-20 minutes)
5. Verify packages on npm:
   ```bash
   npm view @llm-dev-ops/llm-edge-agent-linux-x64
   npm view @llm-dev-ops/llm-edge-agent-linux-arm64
   npm view @llm-dev-ops/llm-edge-agent-darwin-x64
   npm view @llm-dev-ops/llm-edge-agent-darwin-arm64
   npm view @llm-dev-ops/llm-edge-agent-windows-x64
   npm view @llm-dev-ops/llm-edge-agent-windows-arm64
   ```

#### Step 4: Publish Main Package

1. **Wait 60 seconds** after platform packages are published (npm indexing delay)
2. Go to GitHub: **Actions** → **Publish to npm** → **Run workflow**
3. Select options:
   - Branch: `main`
   - Publishing type: **main-only**
   - Version: (leave empty)
4. Click **Run workflow**
5. Wait for completion (typically 2-3 minutes)

#### Step 5: Verify Installation

```bash
# Install globally
npm install -g @llm-dev-ops/llm-edge-agent

# Check version
llm-edge-agent --version

# Test help
llm-edge-agent --help

# Test health check (requires running server)
llm-edge-agent config init
llm-edge-agent start &
sleep 5
llm-edge-agent health

# Cleanup
killall llm-edge-agent
```

#### Step 6: Create GitHub Release

1. Create a git tag:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

2. Create a GitHub release:
   - Go to: Repository → Releases → Draft a new release
   - Choose tag: `v0.2.0`
   - Title: `v0.2.0`
   - Description: Copy from CHANGELOG.md
   - Attach binaries (optional):
     - Download artifacts from GitHub Actions
     - Attach platform binaries to release

### Manual Publishing (Fallback)

If the automated workflow fails, you can publish manually:

#### Step 1: Build All Platform Binaries

```bash
# Install cross-compilation tools
cargo install cross

# Build for all platforms
npm run build:all
```

This will take 20-30 minutes and requires:
- Rust toolchain installed
- `cross` installed for cross-compilation
- Docker running (for cross-compilation)

#### Step 2: Package Platform Binaries

```bash
# Package all platforms
npm run package:all

# Verify packages were created
ls -la npm-packages/
```

Expected output:
```
npm-packages/
├── llm-edge-agent-linux-x64/
├── llm-edge-agent-linux-arm64/
├── llm-edge-agent-darwin-x64/
├── llm-edge-agent-darwin-arm64/
├── llm-edge-agent-windows-x64/
└── llm-edge-agent-windows-arm64/
```

#### Step 3: Publish Platform Packages

```bash
# Login to npm
npm login

# Publish all platform packages
npm run publish:platforms

# This will:
# - Validate npm authentication
# - Publish all 6 platform packages
# - Add 5-second delays between publishes (rate limiting)
# - Display summary report
```

#### Step 4: Publish Main Package

```bash
# Wait 60 seconds for npm indexing
sleep 60

# Publish main package
npm run publish:main
```

#### Step 5: Verify

```bash
npm view @llm-dev-ops/llm-edge-agent
```

## Troubleshooting

### Version Mismatch Error

```
Error: Version mismatch between package.json (0.2.0) and Cargo.toml (0.1.0)
```

**Solution**: Update both files to match:
```bash
npm version 0.2.0 --no-git-tag-version
# Then manually update Cargo.toml
```

### Platform Package Not Found

```
Error: Cannot find module '@llm-dev-ops/llm-edge-agent-linux-x64'
```

**Causes**:
1. Platform packages weren't published
2. npm indexing hasn't completed yet
3. Platform package version doesn't match main package

**Solution**:
```bash
# Check if platform packages exist
npm view @llm-dev-ops/llm-edge-agent-linux-x64

# If not, publish platforms first
npm run publish:platforms

# Wait 60 seconds, then publish main
sleep 60
npm run publish:main
```

### Binary Not Found After Installation

```
Error: Could not find llm-edge-agent binary for linux-x64
```

**Causes**:
1. optionalDependencies installation failed
2. Platform not supported
3. postinstall script failed

**Solution**:
```bash
# Force reinstall
npm install -g @llm-dev-ops/llm-edge-agent --force

# Check which platform package was installed
npm list -g --depth=1 | grep llm-edge-agent

# Manually install platform package
npm install -g @llm-dev-ops/llm-edge-agent-linux-x64
```

### GitHub Actions Workflow Fails

**Common issues**:

1. **NPM_TOKEN not set**:
   - Check: Repository → Settings → Secrets → NPM_TOKEN exists
   - Regenerate token if expired

2. **Build failures**:
   - Check Rust version in workflow (should be stable)
   - Review build logs for specific errors
   - Try local build first: `cargo build --release`

3. **Publish failures**:
   - Check npm authentication: `npm whoami`
   - Verify organization access: `npm org ls @llm-dev-ops`
   - Check if version already exists: `npm view @llm-dev-ops/llm-edge-agent@0.2.0`

### Permission Denied

```
Error: 403 Forbidden - You do not have permission to publish
```

**Solution**:
```bash
# Check current user
npm whoami

# Verify organization membership
npm org ls @llm-dev-ops

# Request access from organization owner
# Or publish to personal scope instead of @llm-dev-ops
```

## Publishing Checklist

Before publishing:

- [ ] Version bumped in both `package.json` and `Cargo.toml`
- [ ] Versions match (prepack validation passes)
- [ ] CHANGELOG.md updated
- [ ] All tests pass locally: `cargo test --workspace`
- [ ] Integration tests pass: `npm test`
- [ ] NPM_TOKEN configured in GitHub Secrets
- [ ] Organization access verified
- [ ] Dry-run completed successfully

Publishing process:

- [ ] Run dry-run workflow
- [ ] Publish platform packages
- [ ] Wait 60 seconds
- [ ] Publish main package
- [ ] Verify installation works
- [ ] Create git tag
- [ ] Create GitHub release
- [ ] Update documentation if needed

## Best Practices

### Version Management

1. **Use semantic versioning**:
   - Patch: Bug fixes (0.1.0 → 0.1.1)
   - Minor: New features (0.1.1 → 0.2.0)
   - Major: Breaking changes (0.2.0 → 1.0.0)

2. **Keep versions synchronized**:
   - Always update both `package.json` and `Cargo.toml`
   - Use the same version for main and platform packages

3. **Tag releases**:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

### Testing Before Publishing

1. **Local testing**:
   ```bash
   # Build locally
   cargo build --release

   # Test CLI
   ./target/release/llm-edge-agent --version

   # Package locally
   npm run package:linux-x64

   # Test package
   cd npm-packages/llm-edge-agent-linux-x64
   npm pack
   npm install -g llm-edge-agent-linux-x64-0.2.0.tgz
   ```

2. **Always run dry-run first**:
   - Use the `dry-run` workflow option
   - Review all output carefully
   - Fix any issues before actual publishing

3. **Smoke tests after publishing**:
   ```bash
   npm install -g @llm-dev-ops/llm-edge-agent
   llm-edge-agent --version
   llm-edge-agent config init --output test.env
   llm-edge-agent --help
   ```

### Rollback Strategy

If you publish a broken version:

1. **Deprecate the version**:
   ```bash
   npm deprecate @llm-dev-ops/llm-edge-agent@0.2.0 "Broken release, use 0.1.0 instead"
   ```

2. **Publish a patch version**:
   - Fix the issue
   - Bump to 0.2.1
   - Publish following normal process

3. **Never unpublish** (unless absolutely necessary):
   - npm doesn't allow unpublishing after 72 hours
   - Unpublishing breaks dependent projects
   - Use deprecation instead

## Automation Improvements

Future enhancements to consider:

1. **Automated version bumping**:
   - Use `npm version` with git hooks
   - Sync to Cargo.toml automatically

2. **Changelog generation**:
   - Use conventional commits
   - Auto-generate CHANGELOG.md

3. **Release automation**:
   - Trigger publish on git tag creation
   - Auto-create GitHub releases

4. **Binary verification**:
   - Add checksums to packages
   - Verify binary integrity after download

## Support

For issues with npm publishing:

1. Check this guide first
2. Review GitHub Actions logs
3. Search existing issues: https://github.com/globalbusinessadvisors/llm-edge-agent/issues
4. Create a new issue with:
   - Version attempting to publish
   - Error messages
   - Workflow run link
   - Steps to reproduce

## Related Documentation

- [package.json Scripts](../package.json)
- [GitHub Actions Workflow](../.github/workflows/npm-publish.yml)
- [Publishing Scripts](../scripts/)
- [NPM Documentation](https://docs.npmjs.com/)
