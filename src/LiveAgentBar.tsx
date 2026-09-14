import { Reveal } from './motion';
import { useEffect,useRef,useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { AgentBar } from './AgentBar';
import { backend,statusLabels,terminalStates,type FleqiController,type ContextSnapshot,type TaskRecord } from './backend';
import { Interaction } from './Interaction';
import { nativeHost } from './native';
export function LiveAgentBar({controller:c}:{controller:FleqiController}){
  const [commandsOpen,setCommandsOpen]=useState(false);const region=useRef<HTMLElement>(null);const request=useRef<{text:string;context:string|null;id:string}|null>(null);const data=c.snapshot;
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
  const task=activeTask??heldTask;const pending=data?.interactions.find(i=>i.taskId===data.activeTaskId);const busy=!!data?.activeTaskId;
  const answerMode=!!pending&&!pending.options?.length;
  const reply=task&&terminalStates.has(task.status)?{status:task.status,message:task.message||'任务已完成',label:statusLabels[task.status]??task.status}:null;
  const contextRef=useRef<ContextSnapshot|null>(data?.context??null);
  useEffect(()=>{contextRef.current=data?.context??null;},[data?.context]);
  // 气泡方向：底栏固定在 Finder 窗口下方不动，气泡按屏幕空间向下或向上展开。
  const [direction,setDirection]=useState<'below'|'above'>('below');
  const spaces=useRef({below:0,above:0});
  const contentRef=useRef(0);
  const measure=useRef<()=>void>(()=>{});
  measure.current=()=>{
    if(!nativeHost||!region.current)return;
    const element=region.current;
    const bar=element.querySelector('.agent-bar');
    if(!bar)return;
    const barRect=bar.getBoundingClientRect();
    let bottom=barRect.bottom;
    for(const child of Array.from(element.children))bottom=Math.max(bottom,child.getBoundingClientRect().bottom);
    const barOffset=Math.max(0,Math.round(barRect.top));
    const extra=Math.max(0,Math.round(bottom-barRect.bottom));
    contentRef.current=extra;
    // 下方放得下就向下展开，否则比较两侧空间后选择更宽的一侧。
    const fitsBelow=extra<=spaces.current.below-8;
    const fitsAbove=extra<=spaces.current.above-8;
    const next:('below'|'above')=fitsBelow?'below':fitsAbove?'above':spaces.current.above>spaces.current.below?'above':'below';
    setDirection(current=>current===next?current:next);
    const offset=next==='above'?extra:0;
    const height=barOffset+barRect.height+extra+18;
    void backend('bar_resize',{height,barHeight:barRect.height,barOffset:offset})
      .then(result=>{
        const value=result as {spaceBelow?:number;spaceAbove?:number}|undefined;
        if(value&&typeof value.spaceBelow==='number'&&typeof value.spaceAbove==='number')spaces.current={below:value.spaceBelow,above:value.spaceAbove};
      })
      .catch(()=>{});
  };
  useEffect(()=>{if(!nativeHost||!region.current)return;const element=region.current;let frame=0;const update=()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(()=>measure.current());};const observer=new ResizeObserver(update);observer.observe(element);for(const child of Array.from(element.children))observer.observe(child);window.addEventListener('resize',update);update();return()=>{observer.disconnect();cancelAnimationFrame(frame);window.removeEventListener('resize',update);};},[!!data]);
  useEffect(()=>{if(!nativeHost)return;let off:(()=>void)|undefined;let active=true;void listen('fleqi:focus',()=>{setHeldTask(null);setBubbleVisible(true);if(contextRef.current?.hostWindow!=='explicit-selection')void c.run('context_capture');}).then(value=>{if(active)off=value;else value();});return()=>{active=false;off?.();};},[c.run]);
  // 选区全自动跟随：底栏存续期间持续轻量探测 Finder（无焦点门槛，用户在
  // Finder 里改选也会被记录），选中变化才真正捕获。
  useEffect(()=>{
    if(!nativeHost)return;
    const timer=setInterval(()=>{
      if(contextRef.current?.hostWindow==='explicit-selection')return;
      void (async()=>{
        try{
          const peeked=await backend<ContextSnapshot>('context_peek');
          const previous=contextRef.current;
          const changed=!previous||previous.hostWindow==='explicit-selection'||previous.directory!==peeked.directory||previous.files.join('|')!==peeked.files.join('|');
          if(changed){
            const captured=await backend<ContextSnapshot>('context_capture');
            contextRef.current=captured;
          }
        }catch{/* 窗口暂时不可读时保留上一个上下文 */}
      })();
    },2000);
    return()=>{clearInterval(timer);};
  },[]);
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
  const submitAnswer=async(text:string)=>{if(!pending)return;await backend('interaction_answer',{id:pending.id,answer:text});await c.refresh();};
  const notice=c.notice??c.error;
  const shownContextFiles=(data.context?.files??[]).length;
  return <main className="native-agent" data-theme={data.settings.appearance.theme}><section ref={region} className="live-region" data-bubble={direction}>
    {direction==='above'&&<>
      <Reveal show={!!pending&&bubbleVisible&&!commandsOpen} edge="bottom"><div className="agent-bubble above" role="alert">{pending&&<Interaction value={pending} controller={c} inline noForm={answerMode}/>}</div></Reveal>
      <Reveal show={!!reply&&bubbleVisible&&!commandsOpen&&!pending} edge="bottom"><div className="agent-bubble above" role="status"><strong><span className={`status-dot ${reply?.status}`}/>{reply?.label}<button className="bubble-close" onClick={()=>{setBubbleVisible(false);setHeldTask(null);}} aria-label="关闭">×</button></strong><p>{reply?.message}</p></div></Reveal>
      <Reveal show={!!notice&&!commandsOpen} edge="bottom"><div className="agent-bubble above" role="alert">{notice}<button className="bubble-close" onClick={()=>c.setNotice(null)} aria-label="关闭">×</button></div></Reveal>
    </>}
    <AgentBar layoutKey={`${direction}:${pending?1:0}:${reply?1:0}:${notice?1:0}:${shownContextFiles}`} onMenuOpenChange={setCommandsOpen} controller={c} appearance={data.settings.appearance} onAppearance={appearance=>void c.save({appearance})} scene={false} live={{busy,onSubmit:submit,onChoose:()=>c.run('choose_files'),onSettings:()=>c.openSettings('general'),onHide:()=>c.run('bar_toggle',{show:false}),answerMode,onSubmitAnswer:submitAnswer,files:data.context?.files??[],onClearFiles:()=>c.run('context_clear')}}/>
    {direction==='below'&&<>
      <Reveal show={!!pending&&bubbleVisible&&!commandsOpen} edge="top"><div className="agent-bubble" role="alert">{pending&&<Interaction value={pending} controller={c} inline noForm={answerMode}/>}</div></Reveal>
      <Reveal show={!!reply&&bubbleVisible&&!commandsOpen&&!pending} edge="top"><div className="agent-bubble" role="status"><strong><span className={`status-dot ${reply?.status}`}/>{reply?.label}<button className="bubble-close" onClick={()=>{setBubbleVisible(false);setHeldTask(null);}} aria-label="关闭">×</button></strong><p>{reply?.message}</p></div></Reveal>
      <Reveal show={!!notice&&!commandsOpen} edge="top"><div className="agent-bubble" role="alert">{notice}<button className="bubble-close" onClick={()=>c.setNotice(null)} aria-label="关闭">×</button></div></Reveal>
    </>}
  </section></main>;
}
