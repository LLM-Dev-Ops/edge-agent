#!/usr/bin/env node

/**
 * Platform Package Generator for @llm-dev-ops/llm-edge-agent
 *
 * This script generates platform-specific npm packages containing
 * the compiled binary for each supported platform.
 *
 * Usage: node scripts/package.js <platform>
 * Example: node scripts/package.js linux-x64
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const MAIN_PACKAGE = require('../package.json');

const PLATFORMS = {
  'linux-x64': {
    target: 'x86_64-unknown-linux-gnu',
    os: 'linux',
    cpu: 'x64',
    binaryName: 'llm-edge-agent'
  },
  'linux-arm64': {
    target: 'aarch64-unknown-linux-gnu',
    os: 'linux',
    cpu: 'arm64',
    binaryName: 'llm-edge-agent'
  },
  'darwin-x64': {
    target: 'x86_64-apple-darwin',
    os: 'darwin',
    cpu: 'x64',
    binaryName: 'llm-edge-agent'
  },
  'darwin-arm64': {
    target: 'aarch64-apple-darwin',
    os: 'darwin',
    cpu: 'arm64',
    binaryName: 'llm-edge-agent'
  },
  'windows-x64': {
    target: 'x86_64-pc-windows-msvc',
    os: 'win32',
    cpu: 'x64',
    binaryName: 'llm-edge-agent.exe'
  },
  'windows-arm64': {
    target: 'aarch64-pc-windows-msvc',
    os: 'win32',
    cpu: 'arm64',
    binaryName: 'llm-edge-agent.exe'
  }
};

/**
 * Create platform-specific package.json
 */
function createPlatformPackageJson(platform, platformInfo) {
  return {
    name: `@llm-dev-ops/llm-edge-agent-${platform}`,
    version: MAIN_PACKAGE.version,
    description: `llm-edge-agent binary for ${platformInfo.os}-${platformInfo.cpu}`,
    license: MAIN_PACKAGE.license,
    repository: MAIN_PACKAGE.repository,
    homepage: MAIN_PACKAGE.homepage,
    bugs: MAIN_PACKAGE.bugs,
    author: MAIN_PACKAGE.author,
    os: [platformInfo.os],
    cpu: [platformInfo.cpu],
    files: ['bin/'],
    main: 'package.json'
  };
}

/**
 * Package a specific platform
 */
function packagePlatform(platform) {
  const platformInfo = PLATFORMS[platform];
  if (!platformInfo) {
    console.error(`Unknown platform: ${platform}`);
    console.error(`Supported platforms: ${Object.keys(PLATFORMS).join(', ')}`);
    process.exit(1);
  }

  console.log(`\nPackaging ${platform}...`);

  // Create package directory
  const pkgDir = path.join(__dirname, '..', 'npm-packages', platform);
  const binDir = path.join(pkgDir, 'bin');

  if (fs.existsSync(pkgDir)) {
    fs.rmSync(pkgDir, { recursive: true, force: true });
  }

  fs.mkdirSync(binDir, { recursive: true });

  // Find binary from cargo build
  const binarySourcePath = path.join(
    __dirname,
    '..',
    'target',
    platformInfo.target,
    'release',
    platformInfo.binaryName
  );

  if (!fs.existsSync(binarySourcePath)) {
    console.error(`\n❌ Binary not found: ${binarySourcePath}`);
    console.error(`\nPlease build first with:`);
    console.error(`  cargo build --release --target ${platformInfo.target}`);
    console.error(`\nOr use cross for cross-compilation:`);
    console.error(`  cross build --release --target ${platformInfo.target}`);
    process.exit(1);
  }

  // Copy binary
  const binaryDestPath = path.join(binDir, platformInfo.binaryName);
  fs.copyFileSync(binarySourcePath, binaryDestPath);

  // Make executable on Unix
  if (platformInfo.os !== 'win32') {
    fs.chmodSync(binaryDestPath, 0o755);
  }

  console.log(`✓ Binary copied: ${binarySourcePath} -> ${binaryDestPath}`);

  // Create package.json
  const packageJson = createPlatformPackageJson(platform, platformInfo);
  const packageJsonPath = path.join(pkgDir, 'package.json');
  fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));

  console.log(`✓ package.json created`);

  // Copy README
  const mainReadmePath = path.join(__dirname, '..', 'README.md');
  const readmePath = path.join(pkgDir, 'README.md');
  if (fs.existsSync(mainReadmePath)) {
    fs.copyFileSync(mainReadmePath, readmePath);
    console.log(`✓ README copied`);
  }

  // Get binary size
  const stats = fs.statSync(binaryDestPath);
  const sizeInMB = (stats.size / (1024 * 1024)).toFixed(2);

  console.log(`\n✓ Package created successfully!`);
  console.log(`  Location: ${pkgDir}`);
  console.log(`  Binary size: ${sizeInMB} MB`);
  console.log(`  Package: @llm-dev-ops/llm-edge-agent-${platform}@${MAIN_PACKAGE.version}`);

  return pkgDir;
}

/**
 * Package all platforms
 */
function packageAll() {
  console.log('Packaging all platforms...\n');

  const packaged = [];

  for (const platform of Object.keys(PLATFORMS)) {
    try {
      const pkgDir = packagePlatform(platform);
      packaged.push({ platform, pkgDir });
    } catch (err) {
      console.error(`Failed to package ${platform}: ${err.message}`);
    }
  }

  console.log(`\n\n✓ Packaged ${packaged.length}/${Object.keys(PLATFORMS).length} platforms`);

  if (packaged.length > 0) {
    console.log('\nPackages ready at:');
    for (const { platform, pkgDir } of packaged) {
      console.log(`  ${platform}: ${pkgDir}`);
    }

    console.log('\nTo publish these packages, run:');
    console.log('  npm run publish:platforms');
  }

  return packaged;
}

/**
 * Main function
 */
function main() {
  const args = process.argv.slice(2);

  if (args.length === 0 || args[0] === 'all') {
    packageAll();
  } else {
    const platform = args[0];
    packagePlatform(platform);
  }
}

// Run if executed directly
if (require.main === module) {
  main();
}

module.exports = { packagePlatform, packageAll, PLATFORMS };
