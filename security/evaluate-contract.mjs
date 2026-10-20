import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';
const require = createRequire(import.meta.url);
const { contract, probeContract } = require('./probe-contract.cjs');
const [baseline, candidate, tools, output] = process.argv.slice(2).map(x => resolve(x));
const fw = await import(pathToFileURL(join(tools, 'node_modules/@metaharness/flywheel/dist/index.js')));
assert.equal(JSON.parse(readFileSync(join(tools, 'node_modules/@metaharness/flywheel/package.json'))).version, '0.1.12');
const hash = x => createHash('sha256').update(x).digest('hex');
const paths = ['cloud-functions/contracts.js', 'cloud-functions/agents/execution-guard.js'];
const source = root => Object.fromEntries(paths.map(path => [path, readFileSync(join(root, path), 'utf8')]));
const digest = root => hash(JSON.stringify(source(root)));
const parentHash = digest(baseline), candidateHash = digest(candidate);
assert.notEqual(parentHash, candidateHash);
const measurements = []; let originalNames;
function execute(root, args) {
  const run = spawnSync(process.execPath, args, { cwd: root, encoding: 'utf8', timeout: 60000, maxBuffer: 8 * 1024 * 1024, env: { PATH: process.env.PATH } });
  assert.equal(run.signal, null); assert.equal(run.status, 0, run.stderr + run.stdout);
  return { exit: run.status, stdout: run.stdout.replace(/\u001b\[[0-9;]*m/g, ''), stderr: run.stderr };
}
function regression(root) {
  const legacy = execute(root, ['test/cloud-functions.test.js']);
  const names = legacy.stdout.split('\n').map(x => x.match(/^\s+✓ (.+)$/)?.[1]).filter(Boolean);
  assert.equal(names.length, 33); assert.match(legacy.stdout, /Results: 33 passed, 0 failed/);
  if (root === baseline) originalNames = names;
  else assert.deepEqual(names, originalNames);
  let native;
  if (root === candidate) { native = execute(root, ['--test', '--test-reporter=tap', 'test/native-contract.test.cjs']); assert.match(native.stdout, /# tests 38\b/); assert.match(native.stdout, /# fail 0\b/); assert.match(native.stdout, /# skipped 0\b/); }
  return { legacyAssertions: names.length, retainedNames: names, nativeTestEntries: native ? 38 : 0, nativeLeafAssertions: native ? 37 : 0, legacy, native };
}
const fingerprint = fw.gateFingerprint(fw.meetsPromotionRule);
const result = await fw.runFlywheelGenerations({
  rootPolicy: { validation: parentHash },
  proposer: async () => ({ value: candidateHash, summary: 'Enforce nested request contracts and execution guard limits without coercion; default absent concurrency to zero', inverse: { path: 'cloud-functions/contracts.js', parentBytes: source(baseline)['cloud-functions/contracts.js'], hash: hash(source(baseline)['cloud-functions/contracts.js']) } }),
  evaluator: async (policy, suite) => {
    const root = policy.validation === parentHash ? baseline : candidate;
    assert.equal(digest(root), policy.validation);
    const report = await probeContract(root, suite.id === 'holdout' ? contract.holdout : contract.anchor);
    const retained = suite.id === 'anchor' ? regression(root) : undefined;
    const wins = report.map(x => x.pass); if (retained) wins.push(true);
    // A returned wrong decision is a failure, not an abstention/no-op. Only
    // requests with no HTTP response count as no-ops. Cost is not measured.
    const score = { primary: wins.filter(Boolean).length / wins.length, noopRate: report.filter(x => x.status === undefined).length / report.length, costPerWin: 0, regressed: suite.id === 'anchor' && wins.some(x => !x), itemWins: wins };
    measurements.push({ source: policy.validation, suite: suite.id, report, retained, score });
    assert.equal(digest(root), policy.validation); return score;
  },
  holdout: { id: 'holdout', items: contract.holdout }, anchor: { id: 'anchor', items: [...contract.anchor, '33 original assertions retained'] },
  maxGenerations: 1, signer: fw.makeSigner(), promotionRule: fw.meetsPromotionRule, dataSource: 'LIVE_FUNCTIONS_FRAMEWORK_HTTP_AND_EXPORTED_POLICY_HANDLERS',
});
const replay = fw.verifyReplayBundle(result.replayBundle, { pinnedGateFingerprint: fingerprint });
assert.equal(replay.pass, true); assert.equal(result.promotions.length, 1);
const evidence = { contract, contractSha256: hash(readFileSync(new URL('./contract.json', import.meta.url))), probeSha256: hash(readFileSync(new URL('./probe-contract.cjs', import.meta.url))), parentHash, candidateHash, rollbackSources: source(baseline), rollbackSha256: parentHash, measurements, fingerprint, replay, replayBundle: result.replayBundle,
  scope: 'Cloud Function validation component, not full repository merge approval or Rust backend/production IAM evidence. Real Functions Framework listeners/parsers and native loopback HTTP; no substituted responses. Existing legacy mocked request/response tests are retention coverage only. No production data/services, deployment or new persistence. costPerWin=0 is a neutral unmeasured input; wrong responses are not labeled no-ops. Exact parent source-map inverse retained; no production rollback performed.' };
writeFileSync(output, JSON.stringify(evidence, null, 2) + '\n');
console.log(JSON.stringify({ promotions: result.promotions.length, replay, measurements: measurements.map(x => ({ suite: x.suite, passes: x.report.filter(y => y.pass).length, total: x.report.length, retained: x.retained?.legacyAssertions, nativeLeafAssertions: x.retained?.nativeLeafAssertions })) }));
