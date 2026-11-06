'use strict';
const {createRequire}=require('node:module');
const {resolve,join}=require('node:path');
const MAX=Number.MAX_SAFE_INTEGER;
async function probe(root){
 const local=createRequire(join(resolve(root),'package.json'));
 const framework=local('@google-cloud/functions-framework');
 const {getTestServer}=local('@google-cloud/functions-framework/testing');
 const name='counter-'+require('node:crypto').randomUUID();
 framework.http(name,local('./index.js').handler);
 const server=getTestServer(name);
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const origin='http://127.0.0.1:'+server.address().port;
 async function check(name,extra,status,verify){
  const response=await fetch(origin+'/v1/edge/circuit-breaker',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({provider_id:'fixture',execution_ref:'fixture',...extra}),signal:AbortSignal.timeout(3000)});
  const body=await response.json();
  return{name,status:response.status,pass:response.status===status&&verify(body),body};
 }
 try{
  const holdout=[];
  for(const state of ['closed','half_open'])for(const kind of ['failure','success']){
   holdout.push(await check(state+' '+kind+' overflow',{current_state:state,[kind+'_count']:MAX,[kind+'_signal']:{}},400,b=>b.success===false&&b.error?.code==='VALIDATION_ERROR'&&!!b.execution_metadata?.trace_id&&!b.data));
  }
  const anchor=[];
  const cases=[
   ['closed no signal',{failure_count:MAX,success_count:MAX},'closed',MAX,MAX],
   ['open failure ignored',{current_state:'open',failure_count:MAX,failure_signal:{}},'open',MAX,0],
   ['closed last safe success',{success_count:MAX-1,success_signal:{}},'closed',0,MAX],
   ['closed last safe failure',{failure_count:MAX-1,failure_signal:{}},'open',MAX,0],
   ['halfopen last safe success',{current_state:'half_open',success_count:MAX-1,success_signal:{}},'closed',0,MAX],
   ['halfopen last safe failure',{current_state:'half_open',failure_count:MAX-1,failure_signal:{}},'open',MAX,0],
   ['normal failure threshold',{failure_count:4,failure_signal:{}},'open',5,0],
   ['normal success threshold',{current_state:'half_open',success_count:2,success_signal:{}},'closed',0,3]
  ];
  for(const [name,input,state,failure,success]of cases)anchor.push(await check(name,input,200,b=>b.data?.new_state===state&&b.data.new_failure_count===failure&&b.data.new_success_count===success&&Number.isSafeInteger(b.data.new_failure_count)&&Number.isSafeInteger(b.data.new_success_count)));
  return{holdout,anchor};
 }finally{server.closeAllConnections();await new Promise(r=>server.close(r));}
}
module.exports={probe};
if(require.main===module)probe(process.argv[2]||'.').then(x=>console.log(JSON.stringify(x))).catch(e=>{console.error(e);process.exitCode=1;});
