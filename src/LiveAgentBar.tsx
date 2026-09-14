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
  // 气泡方向：底栏绝对钉在屏幕底部，气泡一律向上展开。
  const [direction,setDirection]=useState<'below'|'above'>('above');
  const spaces=useRef({below:0,above:0});
  const contentRef=useRef(0);
  const measure=useRef<()=>void>(()=>{});
  // 气泡、提示、斜杠菜单都不参与底栏排版：底栏在网页里的位置只由下面这段
  // 预留空间决定，面板向上长高，底栏在屏幕上永远不动。
  const overlaySelector='.agent-bubble, .context-popover, .slash-menu, .interaction-card';
  const room={top:6,bottom:0};
  measure.current=()=>{
    if(!nativeHost||!region.current)return;
    const element=region.current;
    const bar=element.querySelector('.agent-bar');
    if(!bar)return;
    const barRect=bar.getBoundingClientRect();
    let above=0;
    let below=0;
    for(const node of Array.from(element.querySelectorAll(overlaySelector))){
      const rect=node.getBoundingClientRect();
      if(rect.width<1||rect.height<1)continue;
      if(rect.bottom<=barRect.top+1)above=Math.max(above,barRect.top-rect.top);
      else if(rect.top>=barRect.bottom-1)below=Math.max(below,rect.bottom-barRect.bottom);
    }
    const padTop=Math.round(above)+room.top;
    const padBottom=Math.round(below)+room.bottom;
    element.style.paddingTop=`${padTop}px`;
    element.style.paddingBottom=`${padBottom}px`;
    // 先读一次布局让网页完成重排，再同步改面板：两者落在同一帧，底栏在屏幕上不被推动。
    void element.getBoundingClientRect();
    const barHeight=Math.round(barRect.height);
    // 下方放得下就向下展开，否则比较两侧空间后选择更宽的一侧。
    const fitsBelow=below<=spaces.current.below-8;
    const fitsAbove=above<=spaces.current.above-8;
    const next:('below'|'above')=fitsBelow?'below':fitsAbove?'above':spaces.current.above>spaces.current.below?'above':'below';
    setDirection(current=>current===next?current:next);
    void backend('bar_resize',{height:padTop+barHeight+padBottom,barHeight,barOffset:padTop})
      .then(result=>{
        const value=result as {spaceBelow?:number;spaceAbove?:number}|undefined;
        if(value&&typeof value.spaceBelow==='number'&&typeof value.spaceAbove==='number')spaces.current={below:value.spaceBelow,above:value.spaceAbove};
      })
      .catch(()=>{});
  };
  useEffect(()=>{if(!nativeHost||!region.current)return;const element=region.current;let frame=0;const update=()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(()=>measure.current());};const observer=new ResizeObserver(update);observer.observe(element);for(const child of Array.from(element.children))observer.observe(child);window.addEventListener('resize',update);update();return()=>{observer.disconnect();cancelAnimationFrame(frame);window.removeEventListener('resize',update);};},[!!data]);
  useEffect(()=>{measure.current();},[direction]);
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
  const submitAnswer=async(text:string)=>{if(!pending)return;await backend('interaction_answer',{id:pending.id,answer:text});await c.refresh();};
  const notice=c.notice??c.error;
  return <main className="native-agent" data-theme={data.settings.appearance.theme}><section ref={region} className="live-region" data-bubble={direction}>
    <div className="bar-anchor">
      {direction==='above'&&<div className="bar-overlays above">
        <Reveal show={!!pending&&bubbleVisible&&!commandsOpen} edge="bottom"><div className="agent-bubble above" role="alert">{pending&&<Interaction value={pending} controller={c} inline noForm={answerMode}/>}</div></Reveal>
        <Reveal show={!!reply&&bubbleVisible&&!commandsOpen&&!pending} edge="bottom"><div className="agent-bubble above" role="status"><strong><span className={`status-dot ${reply?.status}`}/>{reply?.label}<button className="bubble-close" onClick={()=>{setBubbleVisible(false);setHeldTask(null);}} aria-label="关闭">×</button></strong><p>{reply?.message}</p></div></Reveal>
        <Reveal show={!!notice&&!commandsOpen} edge="bottom"><div className="agent-bubble above" role="alert">{notice}<button className="bubble-close" onClick={()=>c.setNotice(null)} aria-label="关闭">×</button></div></Reveal>
      </div>}
      <AgentBar layoutKey={`${direction}:${pending?1:0}:${reply?1:0}:${notice?1:0}`} onMenuOpenChange={setCommandsOpen} controller={c} appearance={data.settings.appearance} onAppearance={appearance=>void c.save({appearance})} scene={false} live={{busy,onSubmit:submit,onChoose:()=>c.run('choose_files'),onSettings:()=>c.openSettings('general'),onHide:()=>c.run('bar_toggle',{show:false}),answerMode,onSubmitAnswer:submitAnswer,files:data.context?.files??[],explicit:data.context?.hostWindow==='explicit-selection',onClearFiles:()=>c.run('context_clear')}}/>
      {direction==='below'&&<div className="bar-overlays below">
        <Reveal show={!!pending&&bubbleVisible&&!commandsOpen} edge="top"><div className="agent-bubble" role="alert">{pending&&<Interaction value={pending} controller={c} inline noForm={answerMode}/>}</div></Reveal>
        <Reveal show={!!reply&&bubbleVisible&&!commandsOpen&&!pending} edge="top"><div className="agent-bubble" role="status"><strong><span className={`status-dot ${reply?.status}`}/>{reply?.label}<button className="bubble-close" onClick={()=>{setBubbleVisible(false);setHeldTask(null);}} aria-label="关闭">×</button></strong><p>{reply?.message}</p></div></Reveal>
        <Reveal show={!!notice&&!commandsOpen} edge="top"><div className="agent-bubble" role="alert">{notice}<button className="bubble-close" onClick={()=>c.setNotice(null)} aria-label="关闭">×</button></div></Reveal>
      </div>}
    </div>
  </section></main>;
}
