#!/usr/bin/env node

/**
 * Enhanced CLI for LLM Edge Agent
 * Provides user-friendly commands with proper argument parsing
 */

const { Command } = require('commander');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const http = require('http');

const packageJson = require('../package.json');

// Import the binary resolution logic from cli.js
const { getBinaryPath } = require('./cli');

const program = new Command();

program
  .name('llm-edge-agent')
  .description('Enterprise-grade LLM intercepting proxy with intelligent caching and routing')
  .version(packageJson.version);

// Start command - runs the proxy server
program
  .command('start')
  .description('Start the LLM Edge Agent proxy server')
  .option('-H, --host <host>', 'Server host', '0.0.0.0')
  .option('-p, --port <port>', 'Server port', '8080')
  .option('--metrics-port <port>', 'Metrics port', '9090')
  .option('--enable-l2-cache', 'Enable L2 Redis cache')
  .option('--redis-url <url>', 'Redis connection URL')
  .option('--openai-key <key>', 'OpenAI API key')
  .option('--anthropic-key <key>', 'Anthropic API key')
  .option('--enable-tracing', 'Enable distributed tracing')
  .option('--enable-metrics', 'Enable Prometheus metrics')
  .option('--otlp-endpoint <url>', 'OTLP endpoint for tracing')
  .option('--log-level <level>', 'Log level (error, warn, info, debug, trace)', 'info')
  .option('-d, --daemon', 'Run as daemon (background process)')
  .action((options) => {
    try {
      const chalk = requireChalk();

      console.log(chalk.blue.bold('\n🚀 Starting LLM Edge Agent...\n'));

      // Convert CLI options to environment variables
      const env = {
        ...process.env,
        HOST: options.host,
        PORT: options.port,
        METRICS_PORT: options.metricsPort,
        RUST_LOG: options.logLevel,
      };

      if (options.enableL2Cache) env.ENABLE_L2_CACHE = 'true';
      if (options.redisUrl) env.REDIS_URL = options.redisUrl;
      if (options.openaiKey) env.OPENAI_API_KEY = options.openaiKey;
      if (options.anthropicKey) env.ANTHROPIC_API_KEY = options.anthropicKey;
      if (options.enableTracing) env.ENABLE_TRACING = 'true';
      if (options.enableMetrics) env.ENABLE_METRICS = 'true';
      if (options.otlpEndpoint) env.OTLP_ENDPOINT = options.otlpEndpoint;

      const binaryPath = getBinaryPath();

      console.log(chalk.gray(`Binary: ${binaryPath}`));
      console.log(chalk.gray(`Host: ${options.host}:${options.port}`));
      console.log(chalk.gray(`Metrics: http://${options.host}:${options.metricsPort}/metrics`));
      console.log(chalk.gray(`Log level: ${options.logLevel}\n`));

      const child = spawn(binaryPath, [], {
        stdio: 'inherit',
        env,
        detached: options.daemon
      });

      if (options.daemon) {
        child.unref();
        console.log(chalk.green(`✓ LLM Edge Agent started in daemon mode (PID: ${child.pid})\n`));
        process.exit(0);
      } else {
        console.log(chalk.green('✓ LLM Edge Agent is running\n'));
      }

      child.on('error', (err) => {
        console.error(chalk.red(`\n✗ Failed to start: ${err.message}\n`));
        process.exit(1);
      });

      child.on('exit', (code) => {
        if (code !== 0) {
          console.error(chalk.red(`\n✗ Server exited with code ${code}\n`));
        }
        process.exit(code);
      });
    } catch (err) {
      const chalk = requireChalk();
      console.error(chalk.red(`\n✗ Error: ${err.message}\n`));
      process.exit(1);
    }
  });

// Health command - check server health
program
  .command('health')
  .description('Check health of running LLM Edge Agent instance')
  .option('-H, --host <host>', 'Server host', 'localhost')
  .option('-p, --port <port>', 'Server port', '8080')
  .option('--json', 'Output as JSON')
  .action(async (options) => {
    try {
      const chalk = requireChalk();
      const url = `http://${options.host}:${options.port}/health`;

      const response = await fetchHttp(url);
      const data = JSON.parse(response);

      if (options.json) {
        console.log(JSON.stringify(data, null, 2));
      } else {
        console.log(chalk.green.bold('\n✓ Server is healthy\n'));
        console.log(chalk.gray(`Status: ${data.status || 'ok'}`));
        if (data.version) console.log(chalk.gray(`Version: ${data.version}`));
        if (data.uptime) console.log(chalk.gray(`Uptime: ${data.uptime}`));
        console.log('');
      }

      process.exit(0);
    } catch (err) {
      const chalk = requireChalk();
      console.error(chalk.red(`\n✗ Health check failed: ${err.message}\n`));
      console.error(chalk.gray(`Make sure the server is running at http://${options.host}:${options.port}\n`));
      process.exit(1);
    }
  });

// Config init command - generate example configuration
program
  .command('config')
  .description('Configuration management commands')
  .command('init')
  .description('Generate example configuration file')
  .option('-o, --output <file>', 'Output file path', '.env')
  .option('-f, --force', 'Overwrite existing file')
  .action((options) => {
    try {
      const chalk = requireChalk();

      if (fs.existsSync(options.output) && !options.force) {
        console.error(chalk.red(`\n✗ File ${options.output} already exists. Use --force to overwrite.\n`));
        process.exit(1);
      }

      const template = `# LLM Edge Agent Configuration
# Generated by llm-edge-agent config init

# Server Configuration
HOST=0.0.0.0
PORT=8080
METRICS_PORT=9090

# LLM Provider API Keys (at least one required)
# Get your OpenAI key from: https://platform.openai.com/api-keys
OPENAI_API_KEY=sk-your-openai-key-here

# Get your Anthropic key from: https://console.anthropic.com/settings/keys
ANTHROPIC_API_KEY=sk-ant-your-anthropic-key-here

# Cache Configuration
ENABLE_L2_CACHE=false
REDIS_URL=redis://localhost:6379
L1_CACHE_SIZE=1000
L1_TTL_SECONDS=300
L2_TTL_SECONDS=3600

# Observability
ENABLE_TRACING=false
ENABLE_METRICS=true
OTLP_ENDPOINT=http://localhost:4317

# Logging
RUST_LOG=info,llm_edge_agent=debug
`;

      fs.writeFileSync(options.output, template);
      console.log(chalk.green(`\n✓ Configuration template written to ${options.output}\n`));
      console.log(chalk.gray('Next steps:'));
      console.log(chalk.gray(`  1. Edit ${options.output} and add your API keys`));
      console.log(chalk.gray('  2. Run: llm-edge-agent start\n'));
    } catch (err) {
      const chalk = requireChalk();
      console.error(chalk.red(`\n✗ Failed to create config: ${err.message}\n`));
      process.exit(1);
    }
  });

// Metrics command - fetch current metrics
program
  .command('metrics')
  .description('Fetch Prometheus metrics from running instance')
  .option('-H, --host <host>', 'Server host', 'localhost')
  .option('-p, --port <port>', 'Metrics port', '9090')
  .action(async (options) => {
    try {
      const chalk = requireChalk();
      const url = `http://${options.host}:${options.port}/metrics`;

      const response = await fetchHttp(url);
      console.log(response);
      process.exit(0);
    } catch (err) {
      const chalk = requireChalk();
      console.error(chalk.red(`\n✗ Failed to fetch metrics: ${err.message}\n`));
      console.error(chalk.gray(`Make sure the server is running with metrics enabled at http://${options.host}:${options.port}\n`));
      process.exit(1);
    }
  });

// Benchmark command - run performance benchmarks
program
  .command('benchmark')
  .description('Run performance benchmarks and generate reports')
  .option('-o, --output <dir>', 'Output directory', 'benchmarks/output')
  .option('--json-only', 'Output only JSON results (skip markdown)')
  .action(async (options) => {
    try {
      const chalk = requireChalk();
      const { execSync } = require('child_process');

      console.log(chalk.blue.bold('\n🔬 Running LLM Edge Agent Benchmarks...\n'));

      // Ensure cargo is available
      try {
        execSync('cargo --version', { stdio: 'ignore' });
      } catch (err) {
        console.error(chalk.red('\n✗ Cargo (Rust) is not installed or not in PATH\n'));
        console.error(chalk.gray('Please install Rust from https://rustup.rs/\n'));
        process.exit(1);
      }

      // Run the benchmark binary
      console.log(chalk.gray('Compiling and running benchmarks...\n'));

      try {
        const output = execSync('cargo run --bin benchmark --release', {
          cwd: path.join(__dirname, '..'),
          stdio: 'inherit'
        });

        console.log(chalk.green('\n✓ Benchmarks completed successfully!\n'));
        console.log(chalk.gray(`Results saved to: ${options.output}/\n`));
        console.log(chalk.gray('View summary:'));
        console.log(chalk.gray(`  cat ${options.output}/summary.md\n`));

        process.exit(0);
      } catch (err) {
        console.error(chalk.red('\n✗ Benchmark execution failed\n'));
        console.error(chalk.gray(err.message));
        process.exit(1);
      }
    } catch (err) {
      const chalk = requireChalk();
      console.error(chalk.red(`\n✗ Error: ${err.message}\n`));
      process.exit(1);
    }
  });

// Parse arguments
program.parse();

// Helper: Require chalk with fallback
function requireChalk() {
  try {
    return require('chalk');
  } catch (err) {
    // Fallback if chalk is not available
    return {
      blue: { bold: (s) => s },
      green: { bold: (s) => s },
      red: (s) => s,
      gray: (s) => s,
      green: (s) => s,
    };
  }
}

// Helper: Simple HTTP fetch (Node.js 14+ compatible)
function fetchHttp(url) {
  return new Promise((resolve, reject) => {
    const urlObj = new URL(url);
    const options = {
      hostname: urlObj.hostname,
      port: urlObj.port,
      path: urlObj.pathname,
      method: 'GET',
      timeout: 5000
    };

    const req = http.request(options, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        if (res.statusCode >= 200 && res.statusCode < 300) {
          resolve(data);
        } else {
          reject(new Error(`HTTP ${res.statusCode}: ${res.statusMessage}`));
        }
      });
    });

    req.on('error', reject);
    req.on('timeout', () => {
      req.destroy();
      reject(new Error('Request timeout'));
    });

    req.end();
  });
}
