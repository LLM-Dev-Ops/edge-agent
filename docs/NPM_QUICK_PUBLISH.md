# Quick NPM Publishing Guide

**Fast reference for publishing to npm. For detailed instructions, see [NPM_PUBLISHING.md](./NPM_PUBLISHING.md).**

## Prerequisites

- [ ] NPM_TOKEN added to GitHub Secrets
- [ ] Member of @llm-dev-ops organization
- [ ] Version bumped in `package.json` and `Cargo.toml` (must match)
- [ ] CHANGELOG.md updated

## Publishing Steps

### 1. Prepare Release

```bash
# Update version (example: 0.2.0)
npm version 0.2.0 --no-git-tag-version

# Update Cargo.toml manually
vim Cargo.toml  # Change version = "0.2.0"

# Update CHANGELOG
vim CHANGELOG.md

# Commit
git add package.json Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 0.2.0"
git push origin main
```

### 2. Publish via GitHub Actions

**Option A: Full Publish (Recommended)**

1. Go to: **Actions** → **Publish to npm** → **Run workflow**
2. Select: `publish_type: all`
3. Leave version empty
4. Click **Run workflow**
5. Wait ~20 minutes
6. ✅ Done!

**Option B: Step-by-Step (Safer for First Release)**

1. **Dry Run First**:
   - Actions → Publish to npm → Run workflow
   - Select: `publish_type: dry-run`
   - Review output

2. **Publish Platforms**:
   - Actions → Publish to npm → Run workflow
   - Select: `publish_type: platforms-only`
   - Wait ~15 minutes

3. **Wait for npm indexing**:
   - Wait 60 seconds

4. **Publish Main Package**:
   - Actions → Publish to npm → Run workflow
   - Select: `publish_type: main-only`
   - Wait ~3 minutes

### 3. Verify

```bash
# Install globally
npm install -g @llm-dev-ops/llm-edge-agent

# Test
llm-edge-agent --version
llm-edge-agent --help
```

### 4. Tag Release

```bash
git tag v0.2.0
git push origin v0.2.0
```

### 5. Create GitHub Release

1. Go to: **Releases** → **Draft a new release**
2. Tag: `v0.2.0`
3. Title: `v0.2.0`
4. Description: Copy from CHANGELOG.md
5. Publish

## Manual Publishing (If GitHub Actions Fails)

```bash
# Build all platforms (requires Docker + cross)
npm run build:all

# Package all platforms
npm run package:all

# Login to npm
npm login

# Publish platforms
npm run publish:platforms

# Wait for indexing
sleep 60

# Publish main
npm run publish:main

# Verify
npm view @llm-dev-ops/llm-edge-agent
```

## Common Issues

### Version Mismatch
```
Error: Version mismatch between package.json and Cargo.toml
```
**Fix**: Update both files to match

### Platform Package Not Found
```
Error: Cannot find module '@llm-dev-ops/llm-edge-agent-linux-x64'
```
**Fix**: Publish platforms first, wait 60s, then publish main

### Permission Denied
```
Error: 403 Forbidden
```
**Fix**: Check `npm whoami` and organization membership

## Quick Checklist

Pre-publish:
- [ ] Versions match in package.json and Cargo.toml
- [ ] CHANGELOG.md updated
- [ ] Tests pass: `npm test`
- [ ] NPM_TOKEN in GitHub Secrets
- [ ] Dry-run successful

Publishing:
- [ ] Publish platforms (or "all")
- [ ] Wait 60 seconds
- [ ] Publish main (if not using "all")
- [ ] Verify installation
- [ ] Create git tag
- [ ] Create GitHub release

## Commands Reference

```bash
# Version bump
npm version <major|minor|patch> --no-git-tag-version

# Build
npm run build:all          # All platforms
npm run build:linux        # Linux only
npm run build:macos        # macOS only
npm run build:windows      # Windows only

# Package
npm run package:all        # All platforms
npm run package:linux-x64  # Single platform

# Publish
npm run publish:platforms  # Platform packages
npm run publish:main       # Main package
npm run publish:dry        # Dry run

# Verify
npm view @llm-dev-ops/llm-edge-agent
npm view @llm-dev-ops/llm-edge-agent@latest version
npm view @llm-dev-ops/llm-edge-agent versions --json
```

## Support

Issues? Check [NPM_PUBLISHING.md](./NPM_PUBLISHING.md) for detailed troubleshooting.
