'use strict';
const assert=require('node:assert/strict');
const {test}=require('node:test');
const {resolve}=require('node:path');
const {probe}=require('../security/probe-circuit-counter.cjs');
test('native circuit counter precision',async t=>{
 const result=await probe(resolve(__dirname,'..'));
 for(const row of [...result.holdout,...result.anchor])await t.test(row.name,()=>assert.equal(row.pass,true,JSON.stringify(row)));
});
