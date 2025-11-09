#!/usr/bin/env node

/**
 * Platform Package Publisher for @llm-dev-ops/llm-edge-agent
 *
 * This script publishes all platform-specific packages to npm.
 *
 * Prerequisites:
 * - npm login (must be authenticated)
 * - npm whoami (must have access to @llm-dev-ops org)
 * - npm run package:all (packages must be generated)
 *
 * Usage: node scripts/publish-platforms.js [--dry-run]
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const MAIN_PACKAGE = require('../package.json');

/**
 * Check if user is logged in to npm
 */
function checkNpmAuth() {
  try {
    const whoami = execSync('npm whoami', { encoding: 'utf8' }).trim();
    console.log(`✓ Logged in as: ${whoami}`);
    return whoami;
  } catch (err) {
    console.error('❌ Not logged in to npm. Please run: npm login');
    process.exit(1);
  }
}

/**
 * Check if user has access to @llm-dev-ops org
 */
function checkOrgAccess() {
  try {
    const result = execSync('npm org ls llm-dev-ops', { encoding: 'utf8' });
    console.log('✓ Access to @llm-dev-ops org confirmed');
    return true;
  } catch (err) {
    console.warn('⚠ Warning: Could not verify org access. Continuing anyway...');
    return false;
  }
}

/**
 * Publish a single platform package
 */
function publishPlatform(pkgDir, isDryRun = false) {
  const packageJsonPath = path.join(pkgDir, 'package.json');

  if (!fs.existsSync(packageJsonPath)) {
    console.error(`❌ Package not found: ${pkgDir}`);
    return false;
  }

  const pkgJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  const pkgName = pkgJson.name;

  console.log(`\nPublishing ${pkgName}@${pkgJson.version}...`);

  try {
    const publishCmd = isDryRun
      ? 'npm publish --dry-run --access public'
      : 'npm publish --access public';

    const output = execSync(publishCmd, {
      cwd: pkgDir,
      encoding: 'utf8',
      stdio: 'inherit'
    });

    if (isDryRun) {
      console.log(`✓ Dry run successful for ${pkgName}`);
    } else {
      console.log(`✓ Published ${pkgName}@${pkgJson.version}`);
    }

    return true;
  } catch (err) {
    console.error(`❌ Failed to publish ${pkgName}: ${err.message}`);
    return false;
  }
}

/**
 * Publish all platform packages
 */
function publishAll(isDryRun = false) {
  const npmPackagesDir = path.join(__dirname, '..', 'npm-packages');

  if (!fs.existsSync(npmPackagesDir)) {
    console.error(`❌ npm-packages directory not found: ${npmPackagesDir}`);
    console.error('\nPlease run first: npm run package:all');
    process.exit(1);
  }

  const platforms = fs.readdirSync(npmPackagesDir).filter(name => {
    const fullPath = path.join(npmPackagesDir, name);
    return fs.statSync(fullPath).isDirectory();
  });

  if (platforms.length === 0) {
    console.error('❌ No platform packages found');
    console.error('\nPlease run first: npm run package:all');
    process.exit(1);
  }

  console.log(`Found ${platforms.length} platform packages to publish:`);
  for (const platform of platforms) {
    console.log(`  - ${platform}`);
  }

  if (isDryRun) {
    console.log('\n🏃 Running in DRY RUN mode - no packages will be published\n');
  } else {
    console.log('\n⚠️  WARNING: This will publish packages to npm!\n');
  }

  const published = [];
  const failed = [];

  for (const platform of platforms) {
    const pkgDir = path.join(npmPackagesDir, platform);
    const success = publishPlatform(pkgDir, isDryRun);

    if (success) {
      published.push(platform);
    } else {
      failed.push(platform);
    }

    // Add delay between publishes to avoid rate limiting
    if (!isDryRun && platform !== platforms[platforms.length - 1]) {
      console.log('Waiting 5 seconds before next publish...');
      execSync('sleep 5');
    }
  }

  console.log('\n' + '='.repeat(60));
  console.log('Publication Summary');
  console.log('='.repeat(60));
  console.log(`Total: ${platforms.length}`);
  console.log(`✓ Successful: ${published.length}`);
  console.log(`❌ Failed: ${failed.length}`);

  if (published.length > 0) {
    console.log('\nSuccessfully published:');
    for (const platform of published) {
      console.log(`  ✓ @llm-dev-ops/llm-edge-agent-${platform}@${MAIN_PACKAGE.version}`);
    }
  }

  if (failed.length > 0) {
    console.log('\nFailed to publish:');
    for (const platform of failed) {
      console.log(`  ❌ ${platform}`);
    }
  }

  if (!isDryRun && published.length > 0) {
    console.log('\n' + '='.repeat(60));
    console.log('Next Steps');
    console.log('='.repeat(60));
    console.log('1. Publish the main package:');
    console.log('   npm run publish:main');
    console.log('\n2. Verify installation:');
    console.log('   npx @llm-dev-ops/llm-edge-agent@latest --version');
    console.log('\n3. Create a git tag:');
    console.log(`   git tag v${MAIN_PACKAGE.version}`);
    console.log(`   git push origin v${MAIN_PACKAGE.version}`);
  }

  return { published, failed };
}

/**
 * Main function
 */
function main() {
  const args = process.argv.slice(2);
  const isDryRun = args.includes('--dry-run');

  console.log('Platform Package Publisher');
  console.log('='.repeat(60));
  console.log(`Version: ${MAIN_PACKAGE.version}`);
  console.log(`Mode: ${isDryRun ? 'DRY RUN' : 'PUBLISH'}`);
  console.log('='.repeat(60) + '\n');

  // Check authentication
  checkNpmAuth();
  checkOrgAccess();

  // Publish packages
  const result = publishAll(isDryRun);

  // Exit with error if any failed
  if (result.failed.length > 0) {
    process.exit(1);
  }
}

// Run if executed directly
if (require.main === module) {
  main();
}

module.exports = { publishPlatform, publishAll };
