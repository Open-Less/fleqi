import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {discoverModels,normalizeModels} from '../catalog.mjs';

test('keeps account model IDs and their real thinking and speed choices',()=>{
  const models=normalizeModels([{slug:'account-model',display_name:'Account Model',priority:0,visibility:'list',supported_reasoning_levels:[{effort:'low'},{effort:'high'}],default_reasoning_level:'high',service_tiers:[{id:'priority',name:'Fast'}]}]);
  assert.deepEqual(models[0].thinkingLevels,['low','high']);assert.equal(models[0].defaultThinking,'high');assert.equal(models[0].serviceTiers[0].id,'priority');assert.equal(models[0].capabilitySource,'provider');
  const disabled=normalizeModels([{id:'same-id',reasoning:false}],[{id:'same-id',reasoning:true}]);assert.deepEqual(disabled[0].thinkingLevels,['off']);
});
test('API discovery handles root ports, authentication and all pages without a model ID',async()=>{
  const urls=[];const server=createServer((req,res)=>{urls.push(req.url);assert.equal(req.headers.authorization,'Bearer fixture-key');res.setHeader('content-type','application/json');if(req.url==='/models'){res.writeHead(404);res.end('{}');}else if(req.url.includes('after_id'))res.end(JSON.stringify({data:[{id:'second'}],has_more:false}));else res.end(JSON.stringify({data:[{id:'first'}],has_more:true,last_id:'first'}));});
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  try{const base=`http://127.0.0.1:${server.address().port}`;const result=await discoverModels({profile:{baseUrl:base,protocol:'openai-completions',authType:'api_key'},runtime:{getModels:()=>[]},credentials:{read:async()=>({key:'fixture-key'})},providerId:'p',signal:new AbortController().signal});assert.deepEqual(result.models.map(m=>m.id),['first','second']);assert.equal(result.baseUrl,`${base}/v1`);assert.equal(urls.length,3);assert.deepEqual(result.models[0].thinkingLevels,[]);}finally{await new Promise(resolve=>server.close(resolve));}
});
