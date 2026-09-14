import {test} from 'node:test';
import assert from 'node:assert/strict';
import {Bridge,NativeCredentials} from '../bridge.mjs';

test('pinned SDK loads without a user PI installation or configuration',async()=>{
  const sdk=await import('@earendil-works/pi-coding-agent');assert.equal(typeof sdk.ModelRuntime.create,'function');assert.equal(typeof sdk.createAgentSession,'function');
});

test('credential updates serialize refresh and deletion without logging values',async()=>{
  let stored={type:'oauth',access:'initial'}; const sent=[];
  const bridge=new Bridge(message=>{sent.push(message.type);queueMicrotask(()=>{
    if(message.type==='credential_write') stored=message.credential;
    if(message.type==='credential_delete') stored=undefined;
    bridge.receive({id:message.id,result:message.type==='credential_read'?stored:null});
  });});
  const store=new NativeCredentials(bridge,'p');
  const update=store.modify('p',async value=>{assert.equal(value.access,'initial');return {...value,access:'refreshed'};});
  const remove=store.delete('p');await Promise.all([update,remove]);
  assert.equal(stored,undefined);assert.deepEqual(sent,['credential_read','credential_write','credential_delete']);
});
test('cancelled tool request cannot resolve into another request',async()=>{
  const messages=[];const bridge=new Bridge(m=>messages.push(m));const cancel=new AbortController();
  const pending=bridge.request('tool',{},cancel.signal);cancel.abort();await assert.rejects(pending,/取消/);
  assert.equal(bridge.receive({id:messages[0].id,result:'late'}),false);
});
