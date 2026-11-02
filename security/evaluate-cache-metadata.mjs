import {createRequire} from 'node:module';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const require=createRequire(import.meta.url),{probe}=require('./probe-cache-metadata.cjs');
const [base,candidate,tools,out]=process.argv.slice(2).map(x=>resolve(x));
mkdirSync(out,{recursive:true});
const execute=(cwd,args)=>{const p=spawnSync(args[0],args.slice(1),{cwd,encoding:'utf8',timeout:60000,maxBuffer:8*1024*1024});return {args,status:p.status,signal:p.signal,error:p.error?.message,stdout:p.stdout,stderr:p.stderr};};
const measurements={};
for(const [name,root]of [['baseline',base],['candidate',candidate]]){
 const tests=execute(root,['npm','test']),audit=execute(root,['npm','audit','--json']),cli=execute(root,['npm','run','test:cli']);
 const trials=[];for(let i=0;i<3;i++)trials.push(await probe(root));
 measurements[name]={revision:execute(root,['git','rev-parse','HEAD']).stdout.trim(),tests,audit,cli,trials};
 writeFileSync(join(out,name+'.json'),JSON.stringify(measurements[name],null,2));
}
for(const path of ['test/cloud-functions.test.js','test/native-contract.test.cjs'])if(readFileSync(join(base,path),'utf8')!==readFileSync(join(candidate,path),'utf8'))throw Error('Original assertions changed');
const {runFlywheelGenerations,meetsPromotionRule,makeSigner,verifyReplayBundle,gateFingerprint}=await import(pathToFileURL(join(tools,'node_modules/@metaharness/flywheel/dist/index.js')));
const parent=readFileSync(join(base,'cloud-functions/contracts.js'),'utf8');
const result=await runFlywheelGenerations({
 rootPolicy:{revision:measurements.baseline.revision},
 proposer:async()=>({value:measurements.candidate.revision,summary:'Validate cache metadata and compatible audited dependency patches; broad Rust/CI gates remain blocked',inverse:{path:'cloud-functions/contracts.js',parentBytes:parent,hash:createHash('sha256').update(parent).digest('hex')}}),
 evaluator:async(policy,suite)=>{
  const report=Object.values(measurements).find(x=>x.revision===policy.revision);if(!report)throw Error('Unmeasured revision');
  const records=report.trials.flatMap(x=>x[suite.id]);const wins=records.map(x=>x.pass);
  // Full acceptance was frozen before source edits. Rust/CI remain incomplete,
  // so fail closed. Zero cost/no-op inputs do not claim latency or cost savings.
  return {primary:wins.filter(Boolean).length/wins.length,noopRate:0,costPerWin:0,regressed:true,itemWins:wins};
 },holdout:{id:'holdout',items:Array.from({length:24},(_,i)=>i)},anchor:{id:'anchor',items:Array.from({length:18},(_,i)=>i)},
 promotionRule:meetsPromotionRule,maxGenerations:1,signer:makeSigner(),dataSource:'LIVE_NATIVE_HTTP_SYNTHETIC_PROJECT_FIXTURES',cacheEvaluations:false,
});
const replay=verifyReplayBundle(result.replayBundle,{pinnedGateFingerprint:gateFingerprint(meetsPromotionRule)});
writeFileSync(join(out,'flywheel.json'),JSON.stringify({measurements,replay,replayBundle:result.replayBundle,fullAcceptance:'BLOCKED: Rust and required CI/approvals not all passing'},null,2));
console.log(JSON.stringify({measurements:Object.fromEntries(Object.entries(measurements).map(([k,v])=>[k,{revision:v.revision,test:v.tests.status,audit:v.audit.status,cli:v.cli.status,trials:v.trials.map(t=>({holdout:t.holdout.filter(x=>x.pass).length,anchor:t.anchor.filter(x=>x.pass).length}))}])),promotions:result.replayBundle.verified_improvements,replay}));
if(!replay.pass)process.exitCode=1;
