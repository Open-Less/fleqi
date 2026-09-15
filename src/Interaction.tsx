import { Button } from './components/ui/button';
import { Input } from './components/ui/input';
import { useEffect,useRef,useState } from 'react';
import type { FleqiController,Interaction as InteractionData } from './backend';
import { backend } from './backend';
export function Interaction({value,controller,inline=false,noForm=false}:{value:InteractionData;controller:FleqiController;inline?:boolean;noForm?:boolean}){
  const [answer,setAnswer]=useState('');const [busy,setBusy]=useState(false);const [error,setError]=useState<string|null>(null);const input=useRef<HTMLInputElement>(null);const sending=useRef(false);
  useEffect(()=>{setAnswer('');setBusy(false);setError(null);sending.current=false;if(!noForm)input.current?.focus({preventScroll:true});},[value.id,noForm]);
  const submit=async(text:string)=>{if(sending.current)return;sending.current=true;setBusy(true);setError(null);try{await backend('interaction_answer',{id:value.id,answer:text});await controller.refresh();}catch(error){setError(String(error));}finally{sending.current=false;setBusy(false);}};
  return <section className={`interaction-card ${inline?'inline':''}`} role="region" aria-label="需要你的确认"><span className="section-kicker">{value.kind==='confirmation'?'确认操作':value.kind==='auth'?'账号授权':value.kind==='quit'?'退出应用':'Fleqi'}</span><p>{value.message}</p>
    {value.options?.length?<div className="button-row">{value.options.map(option=><Button variant={option.id==='confirm'?'default':'outline'} key={option.id} disabled={busy} onClick={()=>void submit(option.id)}>{option.label}</Button>)}</div>
      :!noForm&&<form className="answer-form" onSubmit={e=>{e.preventDefault();if(answer.trim())void submit(answer);}}><Input ref={input} aria-label="补充回答" placeholder="在这里补充或修改要求…" value={answer} disabled={busy} onChange={e=>setAnswer(e.target.value)} maxLength={16000} onKeyDown={e=>{if(e.key==='Enter'&&(e.nativeEvent.isComposing||e.keyCode===229))e.preventDefault();}}/><Button disabled={busy||!answer.trim()}>{busy?'发送中…':'回复'}</Button></form>}
    {error&&<p role="alert">{error}</p>}
  </section>;
}
