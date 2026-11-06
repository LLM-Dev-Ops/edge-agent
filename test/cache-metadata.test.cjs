'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {probe}=require('../security/probe-cache-metadata.cjs');
test('cache metadata rejects malformed fields and preserves valid decisions',async t=>{
 const result=await probe(require('node:path').resolve(__dirname,'..'));
 for(const item of [...result.holdout,...result.anchor])await t.test(item.name,()=>assert.equal(item.pass,true,item.error));
 assert.equal(result.holdout.length,8);assert.equal(result.anchor.length,6);
});
