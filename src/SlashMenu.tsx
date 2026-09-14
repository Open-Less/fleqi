import { useEffect } from 'react';
import { backend,thinkingLabels,type FleqiController,type ModelProfile } from './backend';
import { modelChange } from './Models';
export interface SlashOption {id:string;label:string;detail:string;select:()=>Promise<void>}
export function slashOptions(text:string,c:FleqiController|undefined,setText:(v:string)=>void):SlashOption[] {
  const [command,...queryParts]=text.slice(1).split(' ');const query=queryParts.join(' ').toLowerCase();
  const profile=c?.snapshot?.profiles.find(p=>p.id===c.snapshot?.settings.defaultProfileId);
  const model=profile?.models.find(m=>m.id===profile.modelId);
  const save=async(next:ModelProfile)=>{await backend('profile_save',{profile:next,apiKey:null});if(c){if(next.id!==c.snapshot?.settings.defaultProfileId)await c.save({defaultProfileId:next.id});await c.refresh();c.setNotice('已更新，将用于下一次任务');}setText('');};
  if(queryParts.length===0)return [
    {id:'model',label:'/model',detail:'选择连接与模型',select:async()=>setText('/model ')},
    {id:'thinking',label:'/thinking',detail:`思考强度 · ${thinkingLabels[profile?.thinking??'default']??profile?.thinking}`,select:async()=>setText('/thinking ')},
    {id:'speed',label:'/speed',detail:`速度 · ${model?.serviceTiers.find(t=>t.id===profile?.serviceTier)?.name??'标准'}`,select:async()=>setText('/speed ')},
    {id:'settings',label:'/settings',detail:'打开独立设置',select:async()=>{await c?.openSettings('general');setText('');}},
  ].filter(o=>o.id.startsWith(command.toLowerCase()));
  if(command==='model')return (c?.snapshot?.profiles??[]).flatMap(p=>p.models.map(m=>({id:`${p.id}:${m.id}`,label:m.name,detail:`${p.name} · ${m.id}`,select:()=>save(modelChange(p,m.id))}))).filter(o=>`${o.label} ${o.detail}`.toLowerCase().includes(query));
  if(command==='thinking'&&model&&profile)return model.thinkingLevels.map(v=>({id:v,label:thinkingLabels[v]??v,detail:v,select:()=>save({...profile,thinking:v})})).filter(o=>`${o.label} ${o.id}`.toLowerCase().includes(query));
  if(command==='speed'&&model&&profile)return [{id:'default',name:'标准（服务默认）'},...model.serviceTiers].map(t=>({id:t.id,label:t.name,detail:t.id,select:()=>save({...profile,serviceTier:t.id==='default'?null:t.id})})).filter(o=>`${o.label} ${o.id}`.toLowerCase().includes(query));
  return [];
}
export function SlashMenu({options,index,onSelect,onHover,onSettings,busy}:{options:SlashOption[];index:number;onSelect:(o:SlashOption)=>void;onHover:(i:number)=>void;onSettings:()=>void;busy:boolean}){
  useEffect(()=>{document.getElementById(`slash-option-${index}`)?.scrollIntoView({block:'nearest'});},[index]);
  return <div className="slash-menu"><header><span>快捷调整</span><span>↑ ↓ 选择 · Enter 确认 · Esc 关闭</span></header><div id="slash-options" role="listbox" aria-label="斜杠命令">{options.length?options.map((o,i)=><button type="button" id={`slash-option-${i}`} role="option" aria-selected={index===i} key={o.id} disabled={busy} onMouseDown={e=>e.preventDefault()} onMouseEnter={()=>onHover(i)} onClick={()=>onSelect(o)}><span><strong>{o.label}</strong><small className="ml-3">{o.detail}</small></span>{index===i&&<small>↵</small>}</button>):<div className="p-4 text-xs text-muted-foreground">没有可用选项。请先连接模型，或选择支持此参数的模型。<button className="ml-2 underline" type="button" onClick={onSettings}>模型设置</button></div>}</div></div>;
}
