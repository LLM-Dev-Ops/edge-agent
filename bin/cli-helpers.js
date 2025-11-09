/**
 * CLI Helper Functions
 * Binary resolution logic extracted from cli.js for reusability
 */

const path = require('path');
const fs = require('fs');

/**
 * Get the platform-specific binary path
 */
function getBinaryPath() {
  const platform = process.platform;
  const arch = process.arch;

  // Map Node.js platform/arch to our package names
  const platformMap = {
    'linux-x64': '@llm-dev-ops/llm-edge-agent-linux-x64',
    'linux-arm64': '@llm-dev-ops/llm-edge-agent-linux-arm64',
    'darwin-x64': '@llm-dev-ops/llm-edge-agent-darwin-x64',
    'darwin-arm64': '@llm-dev-ops/llm-edge-agent-darwin-arm64',
    'win32-x64': '@llm-dev-ops/llm-edge-agent-windows-x64',
    'win32-arm64': '@llm-dev-ops/llm-edge-agent-windows-arm64',
  };

  const binaryName = platform === 'win32' ? 'llm-edge-agent.exe' : 'llm-edge-agent';
  const platformKey = `${platform}-${arch}`;
  const platformPackage = platformMap[platformKey];

  if (!platformPackage) {
    throw new Error(
      `Unsupported platform: ${platform}-${arch}\n` +
      `Supported platforms: ${Object.keys(platformMap).join(', ')}`
    );
  }

  // Strategy 1: Check optional dependencies (primary method)
  try {
    const optionalDepPath = path.join(
      __dirname,
      '..',
      'node_modules',
      platformPackage,
      'bin',
      binaryName
    );

    if (fs.existsSync(optionalDepPath)) {
      return optionalDepPath;
    }
  } catch (err) {
    // Continue to next strategy
  }

  // Strategy 2: Check postinstall download location
  const postinstallPath = path.join(__dirname, binaryName);
  if (fs.existsSync(postinstallPath)) {
    return postinstallPath;
  }

  // Strategy 3: Check local development build
  const devBuildPaths = [
    // Direct target path
    path.join(__dirname, '..', 'target', 'release', binaryName),
    // Platform-specific target path
    path.join(__dirname, '..', 'target', getPlatformTarget(platform, arch), 'release', binaryName),
  ];

  for (const devPath of devBuildPaths) {
    if (fs.existsSync(devPath)) {
      return devPath;
    }
  }

  // No binary found
  throw new Error(
    `Binary not found for ${platform}-${arch}\n\n` +
    'Please ensure the package was installed correctly. If you are developing locally,\n' +
    'build the binary first with: cargo build --release'
  );
}

/**
 * Get Rust target triple for platform/arch
 */
function getPlatformTarget(platform, arch) {
  const targetMap = {
    'linux-x64': 'x86_64-unknown-linux-gnu',
    'linux-arm64': 'aarch64-unknown-linux-gnu',
    'darwin-x64': 'x86_64-apple-darwin',
    'darwin-arm64': 'aarch64-apple-darwin',
    'win32-x64': 'x86_64-pc-windows-msvc',
    'win32-arm64': 'aarch64-pc-windows-msvc',
  };

  return targetMap[`${platform}-${arch}`] || 'unknown';
}

module.exports = {
  getBinaryPath,
  getPlatformTarget,
};
