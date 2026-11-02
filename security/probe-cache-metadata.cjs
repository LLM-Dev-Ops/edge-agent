'use strict';
const assert = require('node:assert/strict');
const {createRequire} = require('node:module');
const {resolve,join} = require('node:path');
async function probe(root) {
 const local=createRequire(join(resolve(root),'package.json'));
 const framework=local('@google-cloud/functions-framework');
 const {getTestServer}=local('@google-cloud/functions-framework/testing');
 const name='cache-metadata-'+require('node:crypto').randomUUID();
 framework.http(name,local('./index.js').handler);
 const server=getTestServer(name);
 await new Promise(ok=>server.listen(0,'127.0.0.1',ok));
 const holdout=[],anchor=[];
 const cases=[
  ['validity must be boolean',{check_cache:true,existing_entry:{is_valid:'false',is_stale:false}},400],
  ['staleness must be boolean',{check_cache:true,existing_entry:{is_valid:true,is_stale:0}},400],
  ['cache key must be string',{check_cache:true,existing_entry:{is_valid:true,cache_key:{secret:'fixture'}}},400],
  ['layer must be string',{check_cache:true,existing_entry:{is_valid:true,layer:42}},400],
  ['success must be boolean',{is_write_operation:true,response_metadata:{success:'false'}},400],
  ['sensitivity must be boolean',{is_write_operation:true,response_metadata:{success:true,contains_sensitive_data:'false'}},400],
  ['size must be nonnegative',{is_write_operation:true,response_metadata:{success:true,response_size_bytes:-1}},400],
  ['latency must be numeric',{is_write_operation:true,response_metadata:{success:true,latency_ms:'slow'}},400],
  ['valid hit',{check_cache:true,existing_entry:{is_valid:true,is_stale:false,cache_key:'key',layer:'L1',access_count:0}},200,'cache_hit'],
  ['stale miss',{check_cache:true,existing_entry:{is_valid:true,is_stale:true}},200,'cache_miss'],
  ['valid write',{is_write_operation:true,response_metadata:{success:true,contains_sensitive_data:false,response_size_bytes:0,latency_ms:0}},200,'cache_write'],
  ['sensitive bypass',{is_write_operation:true,response_metadata:{success:true,contains_sensitive_data:true}},200,'cache_bypass'],
  ['failed response bypass',{is_write_operation:true,response_metadata:{success:false}},200,'cache_bypass'],
  ['opaque forward fields',{existing_entry:{future_field:{anything:[null]}},response_metadata:{future_field:true}},200,'cache_miss'],
 ];
 try {for(const [name,extra,status,decision]of cases){const result={name,pass:false};try{
  const response=await fetch('http://127.0.0.1:'+server.address().port+'/v1/edge/caching',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({input:{execution_ref:'fixture',request_type:'chat_completion',content_hash:'c',prompt_hash:'p',...extra}}),signal:AbortSignal.timeout(3000)});
  const body=await response.json();result.actualStatus=response.status;assert.equal(response.status,status);if(status===400)assert.equal(body.error.code,'VALIDATION_ERROR');else assert.equal(body.data.decision,decision);result.pass=true;
 }catch(e){result.error=e.message;}(status===400?holdout:anchor).push(result);}}
 finally {server.closeAllConnections();await new Promise(ok=>server.close(ok));}
 return {holdout,anchor};
}
module.exports={probe};
if(require.main===module)probe(process.argv[2]||'.').then(result=>{console.log(JSON.stringify(result));if([...result.holdout,...result.anchor].some(x=>!x.pass))process.exitCode=1;});
