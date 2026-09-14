import { Button } from './components/ui/button';
import { Input } from './components/ui/input';
import { useEffect,useRef,useState } from 'react';
import type { FleqiController,Interaction as InteractionData } from './backend';
export function Interaction({value,controller,inline=false,noForm=false}:{value:InteractionData;controller:FleqiController;inline?:boolean;noForm?:boolean}){
  const [answer,setAnswer]=useState('');const [busy,setBusy]=useState(false);const input=useRef<HTMLInputElement>(null);
  useEffect(()=>{setAnswer('');setBusy(false);if(!noForm)input.current?.focus();},[value.id,noForm]);
  const submit=async(text:string)=>{if(busy)return;setBusy(true);await controller.run('interaction_answer',{id:value.id,answer:text});setBusy(false);};
  return <section className={`interaction-card ${inline?'inline':''}`} role="region" aria-label="需要你的确认"><span className="section-kicker">{value.kind==='confirmation'?'确认操作':value.kind==='auth'?'账号授权':value.kind==='quit'?'退出应用':'agent 提问'}</span><p>{value.message}</p>
    {value.options?.length?<div className="button-row">{value.options.map(option=><Button variant={option.id==='confirm'?'default':'outline'} key={option.id} disabled={busy} onClick={()=>void submit(option.id)}>{option.label}</Button>)}</div>
      :!noForm&&<form className="answer-form" onSubmit={e=>{e.preventDefault();if(answer.trim())void submit(answer);}}><Input ref={input} aria-label="补充回答" value={answer} onChange={e=>setAnswer(e.target.value)} maxLength={16000} onKeyDown={e=>{if(e.key==='Enter'&&(e.nativeEvent.isComposing||e.keyCode===229))e.preventDefault();}}/><Button disabled={busy||!answer.trim()}>提交</Button></form>}
  </section>;
}
