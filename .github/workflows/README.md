# GitHub Actions Workflows

## publish-crates.yml

Automates publishing of all LLM Edge Agent crates to crates.io.

### Prerequisites

1. Add your crates.io API token as a GitHub secret:
   - Go to: https://github.com/globalbusinessadvisors/llm-edge-agent/settings/secrets/actions
   - Create secret named: `CRATES_SECRET`
   - Value: Your crates.io API token from https://crates.io/me
   - ✅ Already configured

### Usage

This workflow is triggered manually via GitHub Actions UI:

1. Go to: https://github.com/globalbusinessadvisors/llm-edge-agent/actions
2. Select "Publish to crates.io" workflow
3. Click "Run workflow"
4. Choose the phase to run:
   - **dry-run** (recommended first): Test without publishing
   - **phase1**: Publish only independent crates (providers, cache, monitoring, security, proxy)
   - **phase2**: Publish only dependent crates (routing, agent) - requires phase1 completed
   - **all**: Publish everything in correct order

### Publishing Order

**Phase 1** (Independent crates - published in parallel):
- llm-edge-providers
- llm-edge-cache
- llm-edge-monitoring
- llm-edge-security
- llm-edge-proxy

**Wait 3 minutes** for crates.io indexing

**Phase 2** (Dependent crates - published sequentially):
- llm-edge-routing (depends on llm-edge-providers)
- llm-edge-agent (depends on all Phase 1 crates)

### Recommended Workflow

1. **First run**: Select `dry-run` to verify everything works
2. **Second run**: Select `phase1` to publish independent crates
3. **Wait**: Give it 3-5 minutes for crates.io to index
4. **Third run**: Select `phase2` to publish dependent crates

OR

1. **All-in-one**: Select `all` to publish everything automatically

### Troubleshooting

**"no matching package found"**
- Phase 2 crates need Phase 1 crates published first
- Wait longer for crates.io indexing (try 5 minutes)

**"authentication failed"**
- Check that `CRATES_SECRET` secret is set correctly
- Verify token is valid at https://crates.io/me

**"rate limit exceeded"**
- Wait a few minutes between runs
- Crates.io has rate limits for publishing

### After Publishing

1. Verify all crates on crates.io
2. Check docs.rs for documentation builds
3. Create GitHub release with tag v0.1.0
4. Update README.md with crates.io badges
