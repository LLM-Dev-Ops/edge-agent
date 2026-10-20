'use strict';
const { writeFileSync } = require('node:fs');
const { resolve } = require('node:path');
const { contract, probeContract } = require('./probe-contract.cjs');
const [baseline, candidate, output] = process.argv.slice(2).map(x => resolve(x));
(async () => {
  const trials = [];
  for (let i = 0; i < 5; i++) for (const variant of i % 2 ? ['candidate', 'baseline'] : ['baseline', 'candidate']) {
    const start = performance.now(); const cases = await probeContract(variant === 'baseline' ? baseline : candidate);
    trials.push({ trial: i, variant, ms: performance.now() - start, cases });
  }
  const summary = Object.fromEntries(['baseline', 'candidate'].map(variant => { const rows = trials.filter(x => x.variant === variant); const times = rows.map(x => x.ms).sort((a,b) => a-b); return [variant, { suiteMedianMs: times[2], holdoutWins: rows.flatMap(x => x.cases).filter(x => contract.holdout.includes(x.name) && x.pass).length, holdoutTotal: 105, anchorWins: rows.flatMap(x => x.cases).filter(x => contract.anchor.includes(x.name) && x.pass).length, anchorTotal: 60 }]; }));
  writeFileSync(output, JSON.stringify({ trials, summary, scope: 'Five alternating HTTP contract trials, fresh loopback listener per trial. Includes framework module warmup/parser/server initialization. Correctness repetitions, not production throughput, speedup or measured dollar cost.' }, null, 2) + '\n');
  console.log(JSON.stringify(summary));
})().catch(e => { console.error(e); process.exitCode = 1; });
