const effortOrder=['off','minimal','low','medium','high','xhigh','max','ultra'];
const effort=value=>value==='none'?'off':value;
const clean=value=>typeof value==='string'?value.trim():'';

export function normalizeModels(rows,builtins=[],protocol='openai-completions'){
  const models=[];const seen=new Set();
  for(const raw of [...rows].sort((a,b)=>(a.priority??999)-(b.priority??999))){
    const id=clean(raw.slug??raw.id??raw.name).replace(/^models\//,'');
    if(!id||id.length>500||seen.has(id)||raw.visibility==='hide'||raw.visibility==='hidden')continue;
    if(raw.supportedGenerationMethods&&!raw.supportedGenerationMethods.some(v=>/generateContent/i.test(v)))continue;
    seen.add(id);
    const known=builtins.find(m=>m.id===id&&m.api===protocol)??builtins.find(m=>m.id===id);
    const advertised=raw.supported_reasoning_levels??raw.supported_reasoning_efforts??raw.reasoning_levels??raw.reasoningLevels;
    let levels=Array.isArray(advertised)?advertised.map(v=>effort(typeof v==='string'?v:v.effort??v.id??v.value)).filter(v=>effortOrder.includes(v)):[];
    let source=levels.length?'provider':'unknown';
    if(!levels.length&&known){
      if(known.reasoning){levels=['low','medium','high'];for(const [key,value] of Object.entries(known.thinkingLevelMap??{})){const mapped=typeof value==='string'?effort(value):key;if(effortOrder.includes(mapped)&&!levels.includes(mapped))levels.push(mapped);}}
      else levels=['off'];source='catalog';
    }
    if(raw.reasoning===false){levels=['off'];source='provider';}
    levels=effortOrder.filter(v=>levels.includes(v));
    const preferred=effort(raw.default_reasoning_level??raw.default_reasoning_effort??raw.defaultThinking);
    const tiers=raw.service_tiers??raw.serviceTiers??raw.additional_speed_tiers??[];
    const serviceTiers=Array.isArray(tiers)?tiers.map(t=>typeof t==='string'?{id:t,name:t==='fast'||t==='priority'?'快速':t}:{id:clean(t.id),name:clean(t.display_name??t.name??t.label)||((t.id==='fast'||t.id==='priority')?'快速':t.id)}).filter(t=>t.id&&t.id!=='default'&&t.id!=='auto'):[];
    models.push({id,name:clean(raw.display_name??raw.displayName??raw.label)||id,description:clean(raw.description).slice(0,500),thinkingLevels:levels,defaultThinking:levels.includes(preferred)?preferred:levels.includes('medium')?'medium':levels[0]??'default',serviceTiers,contextWindow:raw.context_window??raw.inputTokenLimit??known?.contextWindow??128000,maxTokens:raw.max_output_tokens??raw.outputTokenLimit??known?.maxTokens??8192,capabilitySource:source});
  }
  return models;
}

async function requestJson(url,headers,signal){
  const response=await fetch(url,{headers,signal:AbortSignal.any([signal,AbortSignal.timeout(20000)]),redirect:'error'});
  if(!response.ok){const error=new Error(response.status===401||response.status===403?'认证失败，请检查密钥或重新登录。':response.status===404?'服务未提供此模型列表地址，请检查服务地址和协议。':`获取模型失败（HTTP ${response.status}），请稍后重试。`);error.status=response.status;throw error;}
  const contentType=response.headers.get('content-type')??'';
  if(!contentType.includes('json'))throw new Error('服务返回了非 JSON 内容，请确认填写的是 API 地址。');
  const reader=response.body.getReader();let bytes=0;const parts=[];
  for(;;){const {done,value}=await reader.read();if(done)break;bytes+=value.length;if(bytes>8_000_000){await reader.cancel();throw new Error('模型列表过大，请检查服务响应。');}parts.push(Buffer.from(value));}
  return JSON.parse(Buffer.concat(parts).toString('utf8'));
}

export async function discoverModels({profile,runtime,credentials,providerId,signal}){
  const base=profile.baseUrl.replace(/\/+$/,'');const headers={Accept:'application/json'};let urls;let resolvedBaseUrl=base;
  if(profile.authType==='oauth'){
    const auth=await runtime.getAuth(providerId,{signal});if(!auth?.auth?.apiKey)throw new Error('请先登录 Codex 账号。');
    headers.Authorization=`Bearer ${auth.auth.apiKey}`;headers.originator='pi';headers['User-Agent']='Fleqi/0.0.1 (PI 0.85.0)';
    const stored=await credentials.read(providerId,{signal});
    let account=stored?.accountId;
    if(!account){try{account=JSON.parse(Buffer.from(auth.auth.apiKey.split('.')[1],'base64url').toString())['https://api.openai.com/auth']?.chatgpt_account_id;}catch{/* The service reports a missing account route explicitly. */}}
    if(account)headers['chatgpt-account-id']=account;
    urls=[`${base.endsWith('/codex')?base:`${base}/codex`}/models?client_version=0.154.0`];
  }else{
    const stored=await credentials.read(providerId,{signal});const key=stored?.key;
    if(profile.authType==='api_key'&&!key)throw new Error('请先填写 API Key。');
    if(profile.protocol==='anthropic-messages'){
      if(key)headers['x-api-key']=key;headers['anthropic-version']='2023-06-01';urls=[`${base.endsWith('/v1')?base:`${base}/v1`}/models?limit=1000`];
    }else if(profile.protocol==='google-generative-ai'){
      if(key)headers['x-goog-api-key']=key;urls=[`${/\/v1(beta)?$/.test(base)?base:`${base}/v1beta`}/models?pageSize=1000`];
    }else{if(key)headers.Authorization=`Bearer ${key}`;urls=[`${base}/models`];if(!/\/v\d+$/.test(base))urls.push(`${base}/v1/models`);}
  }
  let first;let endpoint;
  for(const url of urls){try{first=await requestJson(url,headers,signal);endpoint=url;break;}catch(error){if(error.status!==404||url===urls.at(-1))throw error;}}
  if(!endpoint)throw new Error('无法获取模型列表。');
  if(profile.protocol.startsWith('openai-')&&profile.authType!=='oauth')resolvedBaseUrl=endpoint.replace(/\/models(?:\?.*)?$/,'');
  const rows=[];const cursors=new Set();let page=first;
  for(let index=0;index<100;index++){
    const items=page.models??page.data;if(!Array.isArray(items))throw new Error('服务响应中没有模型列表。');rows.push(...items);
    if(rows.length>10000)throw new Error('模型数量超过 10000，请检查服务响应。');
    const cursor=page.nextPageToken??(page.has_more?page.last_id:null);if(!cursor)break;
    if(cursors.has(cursor))throw new Error('服务返回了重复分页游标，模型列表未完整获取。');cursors.add(cursor);
    if(index===99)throw new Error('模型列表分页过多。');
    const next=new URL(endpoint);next.searchParams.set(page.nextPageToken?'pageToken':'after_id',cursor);page=await requestJson(next,headers,signal);
  }
  const models=normalizeModels(rows,runtime.getModels(),profile.protocol);
  if(!models.length)throw new Error('此连接没有返回可选的生成模型，请检查账号权限或服务配置。');
  return {models,baseUrl:resolvedBaseUrl,fetchedAt:Date.now()};
}
