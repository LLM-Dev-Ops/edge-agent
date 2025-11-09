#!/usr/bin/env node

/**
 * Prepack Script for @llm-dev-ops/llm-edge-agent
 *
 * This script runs before `npm pack` or `npm publish` to ensure
 * the package is ready for distribution.
 *
 * It performs:
 * 1. Validation checks
 * 2. Version synchronization checks
 * 3. File integrity checks
 */

const fs = require('fs');
const path = require('path');

const PACKAGE_JSON = require('../package.json');

/**
 * Check if Cargo.toml version matches package.json
 */
function checkVersionSync() {
  const cargoTomlPath = path.join(__dirname, '..', 'Cargo.toml');

  if (!fs.existsSync(cargoTomlPath)) {
    console.warn('⚠ Warning: Cargo.toml not found');
    return;
  }

  const cargoToml = fs.readFileSync(cargoTomlPath, 'utf8');
  const versionMatch = cargoToml.match(/^\s*version\s*=\s*"([^"]+)"/m);

  if (versionMatch) {
    const cargoVersion = versionMatch[1];
    if (cargoVersion !== PACKAGE_JSON.version) {
      console.error(
        `\n❌ Version mismatch!\n` +
        `  package.json: ${PACKAGE_JSON.version}\n` +
        `  Cargo.toml:   ${cargoVersion}\n\n` +
        `Please synchronize versions before publishing.`
      );
      process.exit(1);
    }
    console.log(`✓ Version synchronized: ${PACKAGE_JSON.version}`);
  }
}

/**
 * Check required files exist
 */
function checkRequiredFiles() {
  const requiredFiles = ['README.md', 'LICENSE', 'bin/cli.js'];

  let allExist = true;

  for (const file of requiredFiles) {
    const filePath = path.join(__dirname, '..', file);
    if (!fs.existsSync(filePath)) {
      console.error(`❌ Required file missing: ${file}`);
      allExist = false;
    }
  }

  if (!allExist) {
    process.exit(1);
  }

  console.log(`✓ All required files present`);
}

/**
 * Check scripts directory exists
 */
function checkScripts() {
  const scriptsDir = path.join(__dirname, '..');
  const requiredScripts = ['install.js', 'prepack.js'];

  for (const script of requiredScripts) {
    const scriptPath = path.join(scriptsDir, 'scripts', script);
    if (!fs.existsSync(scriptPath)) {
      console.error(`❌ Required script missing: scripts/${script}`);
      process.exit(1);
    }
  }

  console.log(`✓ All scripts present`);
}

/**
 * Validate package.json fields
 */
function validatePackageJson() {
  const required = ['name', 'version', 'description', 'license', 'repository', 'bin'];

  for (const field of required) {
    if (!PACKAGE_JSON[field]) {
      console.error(`❌ Missing required field in package.json: ${field}`);
      process.exit(1);
    }
  }

  // Check bin field
  if (!PACKAGE_JSON.bin['llm-edge-agent']) {
    console.error(`❌ Missing 'llm-edge-agent' entry in bin field`);
    process.exit(1);
  }

  console.log(`✓ package.json validated`);
}

/**
 * Main prepack function
 */
function prepack() {
  console.log('Running prepack checks for @llm-dev-ops/llm-edge-agent...\n');

  validatePackageJson();
  checkVersionSync();
  checkRequiredFiles();
  checkScripts();

  console.log('\n✓ All prepack checks passed!');
  console.log(`\nReady to publish version ${PACKAGE_JSON.version}`);
}

// Run prepack checks
if (require.main === module) {
  try {
    prepack();
  } catch (err) {
    console.error(`\n❌ Prepack failed: ${err.message}`);
    process.exit(1);
  }
}

module.exports = { prepack };
