import { Reveal } from './motion';
import { X } from 'lucide-react';
import { useBarPanelSize } from './useBarPanelSize';
import { useEffect,useRef,useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { AgentBar } from './AgentBar';
import { backend,statusLabels,terminalStates,type FleqiController,type ContextSnapshot,type TaskRecord } from './backend';
import { Interaction } from './Interaction';
import { nativeHost } from './native';
export function LiveAgentBar({controller:c}:{controller:FleqiController}){
  const region=useRef<HTMLElement>(null);const request=useRef<{text:string;context:string|null;id:string}|null>(null);const data=c.snapshot;
  // 每次唤起（fleqi:focus）都是全新会话：上一条任务结果不再随新唤起显示。
  const [heldTask,setHeldTask]=useState<TaskRecord|null>(null);
  const [bubbleVisible,setBubbleVisible]=useState(true);
  const previousActive=useRef<string|null>(null);
  const activeTask=data?.tasks.find(t=>t.id===data.activeTaskId)??null;
  useEffect(()=>{
    const active=data?.activeTaskId??null;
    if(previousActive.current&&!active){
      const finished=data?.tasks.find(t=>t.id===previousActive.current);
      if(finished){setHeldTask(finished);setBubbleVisible(true);}
    }
    if(active)setHeldTask(null);
    previousActive.current=active;
  },[data?.activeTaskId,data?.tasks]);
  const task=activeTask??(previousActive.current?data?.tasks.find(t=>t.id===previousActive.current):null)??heldTask;const pending=data?.interactions.find(i=>i.taskId===data.activeTaskId);const busy=!!data?.activeTaskId;
  const reply=task&&terminalStates.has(task.status)?{status:task.status,message:task.message||'任务已完成',label:statusLabels[task.status]??task.status}:null;
  const contextRef=useRef<ContextSnapshot|null>(data?.context??null);
  useEffect(()=>{contextRef.current=data?.context??null;},[data?.context]);
  useBarPanelSize(region,!!data);
  useEffect(()=>{if(!nativeHost)return;let off:(()=>void)|undefined;let active=true;void listen('fleqi:focus',()=>{setHeldTask(null);setBubbleVisible(true);if(contextRef.current?.hostWindow!=='explicit-selection')void c.run('context_capture');}).then(value=>{if(active)off=value;else value();});return()=>{active=false;off?.();};},[c.run]);
  // 选区跟随：轻量读取 Finder 选中的路径（不解析目录、不校验窗口），
  // 选区一变就立刻做一次完整捕获，界面上大约 0.1 秒内跟着变。
  // 连续几次读到同样的结果就退避到 300ms，避免空闲时高频遍历可访问性树；
  // 一旦结果变化立刻回到 100ms，保证选中后的即时感。
  useEffect(()=>{
    if(!nativeHost)return;
    let stopped=false;let timer:ReturnType<typeof setTimeout>;let same=0;
    const signature=()=>(contextRef.current?.files??[]).join('|');
    const tick=async()=>{
      let changed=false;
      if(contextRef.current?.hostWindow!=='explicit-selection'){
        try{
          const files=await backend<string[]>('selection_peek');
          if(!stopped){
            changed=files.join('|')!==signature();
            if(changed){
              same=0;
              const captured=await backend<ContextSnapshot>('context_capture');
              if(!stopped){contextRef.current=captured;await c.refresh();}
            }else same=Math.min(same+1,3);
          }
        }catch{/* 窗口暂时不可读时保留上一个上下文 */}
      }
      if(!stopped)timer=setTimeout(tick,changed||same<3?100:300);
    };
    timer=setTimeout(tick,50);
    return()=>{stopped=true;clearTimeout(timer);};
  },[c.refresh]);
  // 气泡自动消失：设置常驻（null）时仅提供关闭按钮。
  useEffect(()=>{
    if(!reply||!bubbleVisible)return;
    const seconds=data?.settings.bubbleSeconds??null;
    if(seconds===null)return;
    const timer=setTimeout(()=>setBubbleVisible(false),Math.min(30,Math.max(1,seconds))*1000);
    return()=>{clearTimeout(timer);};
  },[reply?.status,reply?.message,data?.settings.bubbleSeconds]);
  if(!data)return <main className="native-agent"><p className="bar-loading">{c.error??'正在连接 Fleqi…'}</p></main>;
  const submit=async(text:string)=>{const context=data.context?.hostWindow==='explicit-selection'?data.context.id:null;if(!request.current||request.current.text!==text||request.current.context!==context)request.current={text,context,id:crypto.randomUUID()};const task=await backend<TaskRecord>('task_submit',{input:{requestId:request.current.id,prompt:text,contextId:context}});request.current=null;await c.refresh();return task;};
  const notice=c.notice??c.error;
  return <main className="native-agent" data-theme={data.settings.appearance.theme}><section ref={region} className="live-region">
    <AgentBar controller={c} appearance={data.settings.appearance} onAppearance={appearance=>void c.save({appearance})} scene={false}
      live={{busy,task:activeTask,waiting:!!pending,onCancel:()=>c.run('task_cancel'),onSubmit:submit,onChoose:()=>c.run('choose_files'),onSettings:()=>c.openSettings('general'),onHide:()=>c.run('bar_toggle',{show:false}),explicit:data.context?.hostWindow==='explicit-selection',onClearFiles:()=>c.run('context_clear')}}
      conversation={<>
        <Reveal show={!!task&&bubbleVisible} edge="bottom" className="conversation-reveal">
          <div className="conversation-scroll" aria-label="与 Fleqi 的对话">
            <div className="chat-row user"><div className="chat-bubble user"><p>{task?.prompt}</p></div></div>
            {pending&&<div className="chat-row assistant"><div className="chat-bubble assistant"><Interaction value={pending} controller={c} inline/></div></div>}
            {reply&&!pending&&<div className="chat-row assistant"><div className="chat-bubble assistant" role="status">
              <div className="chat-heading"><span className={`status-dot ${reply.status}`}/><span>{reply.label}</span><button className="small-icon" onClick={()=>{setBubbleVisible(false);setHeldTask(null);}} aria-label="关闭对话"><X size={14}/></button></div>
              <p>{reply.message}</p>
            </div></div>}
          </div>
        </Reveal>
        <Reveal show={!!notice} edge="bottom"><div className="panel-notice" role="alert"><span>{notice}</span><button className="small-icon" onClick={()=>c.setNotice(null)} aria-label="关闭提示"><X size={14}/></button></div></Reveal>
      </>}/>
  </section></main>;
}
