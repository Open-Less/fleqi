import { AnimatePresence,AutoHeight,PageTransition,Reveal,motion } from './motion';
import { useRef,useState } from 'react';
import { Check, ChevronRight, KeyRound, Plus, LogIn } from 'lucide-react';
import { backend,thinkingLabels,type FleqiController,type ModelProfile,type Protocol } from './backend';
import { Button } from './components/ui/button';
import { Card } from './components/ui/card';
import { Input } from './components/ui/input';
import { Badge } from './components/ui/badge';
import { Dialog,DialogContent,DialogTitle,DialogDescription } from './components/ui/dialog';
import { ActionButton,Choice,EmptyState } from './ui';
import { AssetIcon } from './icons';
import { Interaction } from './Interaction';
export const protocolNames:Record<Protocol,string>={'openai-codex-responses':'Codex 账号登录','openai-responses':'OpenAI Responses','openai-completions':'OpenAI Chat Completions','anthropic-messages':'Anthropic Messages','google-generative-ai':'Google Gemini'};
const endpoints:Record<Protocol,string>={'openai-codex-responses':'https://chatgpt.com/backend-api','openai-completions':'https://api.openai.com/v1','openai-responses':'https://api.openai.com/v1','anthropic-messages':'https://api.anthropic.com','google-generative-ai':'https://generativelanguage.googleapis.com'};
export function modelChange(profile:ModelProfile,modelId:string):ModelProfile {const model=profile.models.find(m=>m.id===modelId);return {...profile,modelId,thinking:model?.thinkingLevels.includes(profile.thinking)?profile.thinking:model?.defaultThinking??'default',serviceTier:model?.serviceTiers.some(t=>t.id===profile.serviceTier)?profile.serviceTier:null};}
export function ModelParameters({profile,onChange,disabled=false}:{profile:ModelProfile;onChange:(p:ModelProfile)=>void;disabled?:boolean}) {
  const model=profile.models.find(m=>m.id===profile.modelId);
  return <div className="grid grid-cols-2 gap-4"><label className="field-label"><span className="flex items-center gap-2"><AssetIcon name="brain" size={15}/>思考强度</span><Choice label="思考强度" value={profile.thinking} disabled={disabled||!model?.thinkingLevels.length} onChange={thinking=>onChange({...profile,thinking})} options={model?.thinkingLevels.length?model.thinkingLevels.map(v=>({value:v,label:thinkingLabels[v]??v})):[{value:profile.thinking,label:'模型默认'}]}/></label><label className="field-label">速度<Choice label="速度" value={profile.serviceTier??'default'} disabled={disabled||!model?.serviceTiers.length} onChange={v=>onChange({...profile,serviceTier:v==='default'?null:v})} options={[{value:'default',label:'标准（服务默认）'},...(model?.serviceTiers.filter(t=>t.id!=='default').map(t=>({value:t.id,label:t.name}))??[])]}/></label></div>;
}
const ConnectionCard=motion.create(Card);
export function Models({controller:c}:{controller:FleqiController}) {
  const d=c.snapshot!;const [editing,setEditing]=useState<ModelProfile|'new'|null>(null);const [deleting,setDeleting]=useState<ModelProfile|null>(null);
  return <div className="space-y-5"><div className="flex items-center justify-between gap-3"><p className="text-xs text-muted-foreground">连接独立保存，切换后用于新任务。</p><Button size="sm" onClick={()=>setEditing('new')}><AssetIcon name="plusCircleFill" size={16}/>添加连接</Button></div>
    <AnimatePresence initial={false}>{d.profiles.length?d.profiles.map(p=><ConnectionCard key={p.id} layout="position" initial={{opacity:0,y:-8}} animate={{opacity:1,y:0}} exit={{opacity:0,height:0,paddingTop:0,paddingBottom:0}} className="gap-4 p-5 shadow-none"><div className="flex items-center gap-3"><span className="rounded-lg bg-muted p-2.5"><AssetIcon name="customLink" size={18}/></span><div className="min-w-0 flex-1"><h3 className="flex items-center gap-2 text-sm font-medium">{p.name}{p.id===d.settings.defaultProfileId&&<Badge variant="secondary">默认</Badge>}</h3><p className="mt-1 truncate text-xs text-muted-foreground">{p.modelId||'等待获取模型'} · {p.models.length} 个模型</p></div><Button size="sm" variant="outline" onClick={()=>setEditing(p)}>管理</Button></div><div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground"><AssetIcon name="checkmarkCircleFill" size={14}/>{p.authType==='none'?'本地连接':p.hasCredential?'凭据已保存':'尚未登录 / 配置密钥'}<span className="ml-auto">{protocolNames[p.protocol]}</span></div><div className="flex flex-wrap gap-2"><ActionButton size="sm" variant="outline" disabled={!d.native||!p.modelId} action={()=>c.run('profile_run',{id:p.id,mode:'test'},'连接与工具调用检查通过')}>测试连接</ActionButton>{p.id!==d.settings.defaultProfileId&&<ActionButton size="sm" variant="ghost" action={()=>c.save({defaultProfileId:p.id})}>设为默认</ActionButton>}{p.hasCredential&&<ActionButton size="sm" variant="ghost" disabled={!!d.activeTaskId} action={()=>c.run('profile_logout',{id:p.id},'凭据已移除')}>退出连接</ActionButton>}<Button size="sm" variant="ghost" className="ml-auto" onClick={()=>setDeleting(p)} aria-label={`删除 ${p.name}`}><AssetIcon name="trash" size={15}/></Button></div></ConnectionCard>):<Card key="empty" className="shadow-none"><EmptyState icon={<KeyRound size={27}/>} title="连接你的模型" description="登录 Codex，或填写 API 地址与密钥。连接后自动获取可用模型。"><Button size="sm" variant="outline" onClick={()=>setEditing('new')}>添加连接</Button></EmptyState></Card>}</AnimatePresence>
    <p className="text-xs leading-5 text-muted-foreground">凭据保存至系统凭据存储。模型列表和参数以服务返回的信息为准。</p>
    <AnimatePresence>{editing&&<ModelEditor key={editing==='new'?'new':editing.id} controller={c} profile={editing==='new'?undefined:editing} onClose={()=>setEditing(null)}/>}</AnimatePresence>
    <Dialog open={!!deleting} onOpenChange={v=>{if(!v)setDeleting(null);}}><DialogContent><DialogTitle>删除连接？</DialogTitle><DialogDescription>删除“{deleting?.name}”及其 Fleqi 凭据，任务记录会保留。</DialogDescription><div className="flex justify-end gap-2"><Button variant="outline" onClick={()=>setDeleting(null)}>保留</Button><ActionButton action={async()=>{await c.run('profile_delete',{id:deleting?.id},'连接已删除');setDeleting(null);}}>删除连接</ActionButton></div></DialogContent></Dialog>
  </div>;
}
/** 登录期间的授权入口。授权提示必须出现在这个弹窗内部：窗口级提示会被
 *  弹窗遮罩挡住，用户看不到也就无法选择登录方式，登录会一直停住。 */
function AuthPanel({controller:c,busy}:{controller:FleqiController;busy:boolean}) {
  const auth=c.snapshot?.auth??null;
  const prompts=c.snapshot?.interactions.filter(i=>!i.taskId&&i.kind==='auth')??[];
  const [copied,setCopied]=useState(false);
  const url=auth?.url??null;
  // 登录结束（busy 复位）后不再保留授权链接；运行时下发提示前也先不占位。
  if(!busy||(!prompts.length&&!url&&!auth?.userCode))return null;
  const copy=async()=>{if(!url)return;try{await navigator.clipboard.writeText(url);setCopied(true);}catch{c.setNotice('复制失败，请手动选中并复制链接。');}};
  return <section className="auth-panel" role="group" aria-label="Codex 账号登录">
    {!prompts.length&&<span className="section-kicker">Codex 账号登录</span>}
    {prompts.map(prompt=><Interaction key={prompt.id} value={prompt} controller={c}/>)}
    {url&&<div className="auth-links"><p className="auth-hint">{auth?.instructions??'已在浏览器打开授权页面。完成登录后会自动返回 Fleqi 并获取模型。'}</p>
      <div className="button-row"><ActionButton size="sm" variant="outline" action={()=>c.run('auth_open',{url:null},'已重新打开授权页')}><AssetIcon name="customLink" size={14}/>重新打开授权页</ActionButton><Button size="sm" variant="ghost" type="button" onClick={()=>void copy()}>{copied?'已复制链接':'复制链接'}</Button></div>
      <code className="auth-url">{url}</code></div>}
    {auth?.userCode&&<p className="auth-hint">设备码 <strong>{auth.userCode}</strong>：已在浏览器打开设备码页面，在其中输入即可完成登录。</p>}
  </section>;
}
function ModelEditor({profile,controller:c,onClose}:{profile?:ModelProfile;controller:FleqiController;onClose:()=>void}) {
  const [draft,setDraft]=useState<ModelProfile>(profile??{id:crypto.randomUUID(),name:'Codex',protocol:'openai-codex-responses',baseUrl:endpoints['openai-codex-responses'],modelId:'',thinking:'default',authType:'oauth',hasCredential:false,models:[],modelsFetchedAt:null,serviceTier:null});
  const [key,setKey]=useState('');const [error,setError]=useState<string|null>(null);const [busy,setBusy]=useState(false);const [filter,setFilter]=useState('');const running=useRef(false);
  const patch=(p:Partial<ModelProfile>)=>setDraft(v=>({...v,...p}));
  const changeProtocol=(protocol:Protocol)=>patch({protocol,name:protocolNames[protocol].replace(' 账号登录',''),baseUrl:endpoints[protocol],authType:protocol==='openai-codex-responses'?'oauth':'api_key',modelId:'',models:[],modelsFetchedAt:null,thinking:'default',serviceTier:null,hasCredential:false});
  const connect=async()=>{if(running.current)return;running.current=true;setBusy(true);setError(null);try{
    await backend('profile_save',{profile:draft,apiKey:key||null});setKey('');
    const result=await backend<{profile?:ModelProfile;result:{catalogError?:string}}>('profile_run',{id:draft.id,mode:draft.authType==='oauth'&&!draft.hasCredential?'login':'models'});
    if(result.profile)setDraft({...result.profile,hasCredential:draft.authType!=='none'});
    if(result.result?.catalogError)throw new Error(`已登录，模型获取失败：${result.result.catalogError}。可点击重新获取。`);
    c.setNotice('模型列表已更新');
  }catch(e){setError(String(e));}finally{running.current=false;setBusy(false);const fresh=await c.refresh();if(fresh){const saved=fresh.profiles.find(p=>p.id===draft.id);if(saved)setDraft(v=>({...v,hasCredential:saved.hasCredential}));}}};
  const save=async()=>{setBusy(true);setError(null);try{await backend('profile_save',{profile:draft,apiKey:key||null});setKey('');await c.refresh();c.setNotice('连接与模型已保存');onClose();}catch(e){setError(String(e));}finally{setBusy(false);}};
  const close=async()=>{if(busy)await c.run('runtime_cancel');onClose();};
  const filtered=draft.models.filter(m=>`${m.name} ${m.id}`.toLowerCase().includes(filter.toLowerCase()));
  return <Dialog open onOpenChange={v=>{if(!v)void close();}}><DialogContent className="model-editor sm:max-w-xl"><DialogTitle>{profile?'管理连接':'添加连接'}</DialogTitle><DialogDescription>连接服务，自动获取模型，再选择任务使用的模型。</DialogDescription><AuthPanel controller={c} busy={busy}/><AutoHeight className="model-body-size"><div className="model-editor-body space-y-4">
    <label className="field-label">连接方式<Choice label="连接方式" value={draft.protocol} disabled={busy} onChange={v=>changeProtocol(v as Protocol)} options={Object.entries(protocolNames).map(([value,label])=>({value,label}))}/></label>
    <label className="field-label">连接名称<Input aria-label="连接名称" value={draft.name} maxLength={100} disabled={busy} onChange={e=>patch({name:e.target.value})}/></label>
    <AutoHeight><PageTransition page={draft.authType==='oauth'?'oauth':'api'} order={['oauth','api']} className="model-auth-fields space-y-4">{draft.authType!=='oauth'?<><label className="field-label">API 服务地址<Input aria-label="API 服务地址" value={draft.baseUrl} disabled={busy} placeholder="https://api.example.com/v1 或 http://localhost:端口/v1" onChange={e=>patch({baseUrl:e.target.value,models:[],modelId:''})}/></label><label className="field-label">认证方式<Choice label="认证方式" value={draft.authType} disabled={busy} onChange={v=>patch({authType:v as 'api_key'|'none'})} options={[{value:'api_key',label:'API Key'},{value:'none',label:'本地服务，无密钥'}]}/></label><Reveal show={draft.authType==='api_key'}><label className="field-label">API Key<Input aria-label="API Key" type="password" autoComplete="new-password" disabled={busy||!c.snapshot?.native} value={key} placeholder={draft.hasCredential?'留空保留已保存的密钥':'只保存至系统凭据存储'} onChange={e=>setKey(e.target.value)}/></label></Reveal></>:<div className="flex items-start gap-3 rounded-lg bg-muted p-4 text-sm"><LogIn size={18} className="mt-0.5 shrink-0"/><div>使用 Codex 账号<p className="mt-1 text-xs leading-5 text-muted-foreground">在浏览器完成登录后，自动加载账号可用的模型。无需填写服务地址或模型 ID。</p></div></div>}</PageTransition></AutoHeight>
    <Button className="w-full" variant={draft.models.length?'outline':'default'} disabled={busy||!c.snapshot?.native||!draft.name.trim()} aria-busy={busy} onClick={()=>void connect()}>{busy?<AssetIcon name="clockRotate" size={14} className="animate-spin"/>:draft.models.length?<AssetIcon name="clockRotate" size={14}/>:<AssetIcon name="customLink" size={14}/>}{busy?(draft.authType==='oauth'?'等待登录并获取模型…':'正在获取模型…'):draft.models.length?'重新获取模型':draft.authType==='oauth'?'登录 Codex 并获取模型':'连接并获取模型'}</Button>
    <Reveal show={draft.models.length>0}><section className="space-y-3"><div className="flex items-center justify-between text-sm font-medium">选择模型<Badge variant="secondary">{draft.models.length} 个</Badge></div><div className="relative"><AssetIcon name="magnifyingglass" size={15} className="absolute top-2.5 left-3 text-muted-foreground"/><Input className="pl-9" aria-label="搜索模型" placeholder="搜索模型名称或 ID" value={filter} onChange={e=>setFilter(e.target.value)}/></div><AutoHeight className="model-options"><div className="model-options-content" role="listbox" aria-label="可用模型">{filtered.map(m=><button role="option" aria-selected={m.id===draft.modelId} key={m.id} className="model-option" disabled={busy} onClick={()=>setDraft(v=>modelChange(v,m.id))}><span className="min-w-0"><strong>{m.name}</strong><small>{m.id}</small></span>{m.id===draft.modelId?<Check size={16}/>:<ChevronRight size={14}/>}</button>)}{!filtered.length&&<p className="p-4 text-xs text-muted-foreground">没有匹配的模型</p>}</div></AutoHeight><ModelParameters profile={draft} onChange={setDraft} disabled={busy}/></section></Reveal>
    <Reveal show={!!error}><p className="rounded-lg border border-foreground/30 p-3 text-xs leading-6" role="alert">{error}</p></Reveal>
    {!c.snapshot?.native&&<p className="text-xs text-muted-foreground">账号连接和密钥录入在 Fleqi 原生应用中使用。</p>}
  </div></AutoHeight><div className="flex justify-end gap-2 border-t pt-4"><Button variant="outline" onClick={()=>void close()}>{busy?'取消操作':'取消'}</Button><Button disabled={busy||!draft.modelId||!c.snapshot?.native} onClick={()=>void save()}>保存连接</Button></div></DialogContent></Dialog>;
}
