#!/usr/bin/env node

/**
 * Postinstall Script for @llm-dev-ops/llm-edge-agent
 *
 * This script provides a fallback binary installation mechanism when
 * optionalDependencies fail to install (e.g., due to --ignore-optional flag).
 *
 * Strategy:
 * 1. Check if binary is already available from optionalDependencies
 * 2. If not, attempt to download from GitHub releases
 * 3. Extract and make executable
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const https = require('https');
const { execSync } = require('child_process');

const PACKAGE_JSON = require('../package.json');
const VERSION = PACKAGE_JSON.version;

const PLATFORM_MAP = {
  'linux': { 'x64': 'linux-x64', 'arm64': 'linux-arm64' },
  'darwin': { 'x64': 'darwin-x64', 'arm64': 'darwin-arm64' },
  'win32': { 'x64': 'windows-x64', 'arm64': 'windows-arm64' }
};

const RUST_TARGET_MAP = {
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'windows-x64': 'x86_64-pc-windows-msvc',
  'windows-arm64': 'aarch64-pc-windows-msvc'
};

/**
 * Get the current platform identifier
 */
function getPlatformIdentifier() {
  const platform = os.platform();
  const arch = os.arch();
  return PLATFORM_MAP[platform]?.[arch];
}

/**
 * Get the binary name for the current platform
 */
function getBinaryName() {
  const platform = os.platform();
  return platform === 'win32' ? 'llm-edge-agent.exe' : 'llm-edge-agent';
}

/**
 * Check if binary already exists from optionalDependencies
 */
function checkOptionalDependency(platformId) {
  const pkgName = `@llm-dev-ops/llm-edge-agent-${platformId}`;

  try {
    const pkgPath = require.resolve(`${pkgName}/package.json`);
    const pkgDir = path.dirname(pkgPath);
    const binaryPath = path.join(pkgDir, 'bin', getBinaryName());

    if (fs.existsSync(binaryPath)) {
      console.log(`✓ Binary already installed via optionalDependencies: ${pkgName}`);
      return true;
    }
  } catch (err) {
    // Package not found
  }

  return false;
}

/**
 * Download file from URL
 */
function download(url, dest) {
  return new Promise((resolve, reject) => {
    console.log(`Downloading from ${url}...`);

    const file = fs.createWriteStream(dest);

    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        https.get(response.headers.location, (redirectResponse) => {
          redirectResponse.pipe(file);
          file.on('finish', () => {
            file.close();
            resolve();
          });
        }).on('error', (err) => {
          fs.unlink(dest, () => {});
          reject(err);
        });
      } else if (response.statusCode === 200) {
        response.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve();
        });
      } else {
        reject(new Error(`Download failed with status ${response.statusCode}`));
      }
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

/**
 * Download binary from GitHub releases
 */
async function downloadFromGitHub(platformId) {
  const rustTarget = RUST_TARGET_MAP[platformId];
  const assetName = platformId.includes('darwin')
    ? `llm-edge-agent-darwin-${platformId.split('-')[1]}`
    : `llm-edge-agent-${platformId.replace('-', '-')}`;

  // GitHub release URL format
  const releaseUrl = `https://github.com/globalbusinessadvisors/llm-edge-agent/releases/download/v${VERSION}/${assetName}`;

  const binDir = path.join(__dirname, '..', 'bin');
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  const binaryName = getBinaryName();
  const tempFile = path.join(binDir, `${binaryName}.tmp`);
  const finalPath = path.join(binDir, binaryName);

  try {
    await download(releaseUrl, tempFile);

    // Move to final location
    fs.renameSync(tempFile, finalPath);

    // Make executable on Unix-like systems
    if (os.platform() !== 'win32') {
      fs.chmodSync(finalPath, 0o755);
    }

    console.log(`✓ Binary installed to ${finalPath}`);
    return true;
  } catch (err) {
    console.error(`Failed to download binary: ${err.message}`);
    return false;
  }
}

/**
 * Check if running in CI environment
 */
function isCI() {
  return process.env.CI === 'true' ||
         process.env.CONTINUOUS_INTEGRATION === 'true' ||
         process.env.GITHUB_ACTIONS === 'true';
}

/**
 * Main installation function
 */
async function install() {
  console.log('Installing llm-edge-agent binary...');

  const platformId = getPlatformIdentifier();
  if (!platformId) {
    const platform = os.platform();
    const arch = os.arch();
    console.warn(
      `⚠ Unsupported platform: ${platform}-${arch}\n` +
      `Supported platforms: linux-x64, linux-arm64, darwin-x64, darwin-arm64, windows-x64, windows-arm64\n` +
      `You will need to build from source using: cargo build --release`
    );
    process.exit(0); // Don't fail installation
  }

  // Check if already installed via optionalDependencies
  if (checkOptionalDependency(platformId)) {
    process.exit(0);
  }

  console.log(`Platform detected: ${platformId}`);
  console.log('optionalDependencies not available, attempting fallback installation...');

  // Try to download from GitHub releases
  const success = await downloadFromGitHub(platformId);

  if (!success) {
    console.warn(
      `\n⚠ Warning: Could not install binary automatically.\n` +
      `\nPlease try one of the following:\n` +
      `  1. Install with optionalDependencies: npm install --include=optional\n` +
      `  2. Build from source: cargo build --release\n` +
      `  3. Download manually from: https://github.com/globalbusinessadvisors/llm-edge-agent/releases\n`
    );

    // Don't fail in CI environments to allow custom build steps
    if (isCI()) {
      console.log('Continuing in CI environment...');
      process.exit(0);
    }

    // Allow installation to continue in development
    process.exit(0);
  }
}

// Run installation
if (require.main === module) {
  install().catch((err) => {
    console.error(`Installation failed: ${err.message}`);
    process.exit(0); // Don't fail npm install
  });
}

module.exports = { install };
