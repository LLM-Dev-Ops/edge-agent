#!/usr/bin/env node

/**
 * LLM Edge Agent CLI Wrapper
 *
 * This script locates and executes the platform-specific binary for llm-edge-agent.
 * It handles binary resolution from optionalDependencies or postinstall downloads.
 */

const { spawn } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');

/**
 * Platform and architecture mapping
 */
const PLATFORM_MAP = {
  'linux': { 'x64': 'linux-x64', 'arm64': 'linux-arm64' },
  'darwin': { 'x64': 'darwin-x64', 'arm64': 'darwin-arm64' },
  'win32': { 'x64': 'windows-x64', 'arm64': 'windows-arm64' }
};

/**
 * Rust target triple to platform mapping
 */
const TARGET_MAP = {
  'x86_64-unknown-linux-gnu': 'linux-x64',
  'aarch64-unknown-linux-gnu': 'linux-arm64',
  'x86_64-apple-darwin': 'darwin-x64',
  'aarch64-apple-darwin': 'darwin-arm64',
  'x86_64-pc-windows-msvc': 'windows-x64',
  'aarch64-pc-windows-msvc': 'windows-arm64'
};

/**
 * Get the current platform identifier
 */
function getPlatformIdentifier() {
  const platform = os.platform();
  const arch = os.arch();

  const platformId = PLATFORM_MAP[platform]?.[arch];
  if (!platformId) {
    throw new Error(
      `Unsupported platform: ${platform}-${arch}\n` +
      `Supported platforms: linux-x64, linux-arm64, darwin-x64, darwin-arm64, windows-x64, windows-arm64`
    );
  }

  return platformId;
}

/**
 * Get the binary name for the current platform
 */
function getBinaryName() {
  const platform = os.platform();
  return platform === 'win32' ? 'llm-edge-agent.exe' : 'llm-edge-agent';
}

/**
 * Locate the binary from optionalDependencies
 */
function findOptionalDependencyBinary(platformId) {
  const pkgName = `@llm-dev-ops/llm-edge-agent-${platformId}`;
  const binaryName = getBinaryName();

  try {
    // Try to resolve from node_modules
    const pkgPath = require.resolve(`${pkgName}/package.json`);
    const pkgDir = path.dirname(pkgPath);
    const binaryPath = path.join(pkgDir, 'bin', binaryName);

    if (fs.existsSync(binaryPath)) {
      return binaryPath;
    }
  } catch (err) {
    // Package not found in optionalDependencies
  }

  return null;
}

/**
 * Locate the binary from postinstall download
 */
function findPostinstallBinary() {
  const binaryName = getBinaryName();
  const binaryPath = path.join(__dirname, '..', 'bin', binaryName);

  if (fs.existsSync(binaryPath)) {
    return binaryPath;
  }

  return null;
}

/**
 * Locate the binary from cargo build (development mode)
 */
function findDevelopmentBinary() {
  const binaryName = getBinaryName();
  const possiblePaths = [
    path.join(__dirname, '..', 'target', 'release', binaryName),
    path.join(__dirname, '..', 'target', 'debug', binaryName)
  ];

  for (const binPath of possiblePaths) {
    if (fs.existsSync(binPath)) {
      return binPath;
    }
  }

  return null;
}

/**
 * Get the path to the llm-edge-agent binary
 */
function getBinaryPath() {
  const platformId = getPlatformIdentifier();

  // Strategy 1: Check optionalDependencies (primary)
  let binaryPath = findOptionalDependencyBinary(platformId);
  if (binaryPath) {
    return binaryPath;
  }

  // Strategy 2: Check postinstall download (fallback)
  binaryPath = findPostinstallBinary();
  if (binaryPath) {
    return binaryPath;
  }

  // Strategy 3: Check cargo build (development)
  binaryPath = findDevelopmentBinary();
  if (binaryPath) {
    return binaryPath;
  }

  // No binary found
  throw new Error(
    `Could not find llm-edge-agent binary for ${platformId}\n\n` +
    `Installation may have failed. Please try:\n` +
    `  1. npm install --force\n` +
    `  2. cargo build --release (if you have Rust installed)\n` +
    `  3. Report an issue at https://github.com/globalbusinessadvisors/llm-edge-agent/issues`
  );
}

/**
 * Execute the binary with the provided arguments
 */
function executeBinary() {
  try {
    const binaryPath = getBinaryPath();

    // Ensure binary is executable (Unix-like systems)
    if (os.platform() !== 'win32') {
      try {
        fs.chmodSync(binaryPath, 0o755);
      } catch (err) {
        // Ignore chmod errors
      }
    }

    // Spawn the binary with all arguments passed to this script
    const child = spawn(binaryPath, process.argv.slice(2), {
      stdio: 'inherit',
      env: process.env
    });

    // Forward exit code
    child.on('exit', (code, signal) => {
      if (signal) {
        process.kill(process.pid, signal);
      } else {
        process.exit(code || 0);
      }
    });

    // Handle errors
    child.on('error', (err) => {
      console.error(`Failed to execute binary: ${err.message}`);
      process.exit(1);
    });
  } catch (err) {
    console.error(err.message);
    process.exit(1);
  }
}

// Execute if run directly
if (require.main === module) {
  executeBinary();
}

module.exports = { getBinaryPath, getPlatformIdentifier };
