import { Reveal } from './motion';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import { ArrowUpRight, File, LoaderCircle, X } from 'lucide-react';
import { BorderBeam } from 'border-beam';
import { AssetIcon,SettingsIcon,SvgImage } from './icons';
import { SlashMenu,slashOptions,type SlashOption } from './SlashMenu';
import type { FleqiController } from './backend';
import { operationLabels } from './backend';
import type { Appearance } from './appearance';
import { getMaterialSupport, nativeHost, updateNativeMaterial, type MaterialSupport } from './native';
import { AppearancePanel } from './AppearancePanel';

interface Props { appearance: Appearance; onAppearance: (value: Partial<Appearance>) => void; scene: boolean; layoutKey?:string; onMenuOpenChange?:(open:boolean)=>void; controller?:FleqiController; live?: {busy:boolean;files?:string[];onClearFiles?:()=>Promise<unknown>;onSubmit:(text:string)=>Promise<unknown>;onChoose:()=>Promise<unknown>;onSettings:()=>Promise<unknown>;onHide:()=>Promise<unknown>;answerMode?:boolean;onSubmitAnswer?:(text:string)=>Promise<unknown>} }

export function AgentBar({ appearance, onAppearance, scene, layoutKey, live, controller,onMenuOpenChange }: Props) {
  const bar = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const settings = useRef<HTMLButtonElement>(null);
  const composing = useRef(false);
  const submitting = useRef(false);
  const [text, setText] = useState('');
  const [menuIndex,setMenuIndex]=useState(0);
  const [menuDismissed,setMenuDismissed]=useState(false);
  const [choosing,setChoosing]=useState(false);
  const [dispatching,setDispatching]=useState(false);
  const [commandBusy,setCommandBusy]=useState(false);
  const [reducedMotion,setReducedMotion]=useState(()=>matchMedia('(prefers-reduced-motion: reduce)').matches);
  useEffect(()=>{const query=matchMedia('(prefers-reduced-motion: reduce)');const change=()=>setReducedMotion(query.matches);query.addEventListener('change',change);return()=>query.removeEventListener('change',change);},[]);
  const menuOpen=text.startsWith('/')&&!menuDismissed&&!live?.busy;
  useEffect(()=>onMenuOpenChange?.(menuOpen),[menuOpen,onMenuOpenChange]);
  const options=slashOptions(text,controller,setText);
  useEffect(()=>setMenuIndex(0),[text]);
  const selectCommand=async(option:SlashOption)=>{if(commandBusy)return;setCommandBusy(true);try{await option.select();setNotice(null);}catch(e){setNotice(String(e));}finally{setCommandBusy(false);input.current?.focus();}};
  const [files, setFiles] = useState<string[]>([]);
  const [open, setOpen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [support, setSupport] = useState<MaterialSupport>({ native: false, liquid: false, reduceTransparency: false });
  const [nativeMaterial, setNativeMaterial] = useState(false);

  useEffect(() => {
    const refresh = () => getMaterialSupport().then(setSupport).catch(() => {
      setNotice('系统材质暂不可用，已使用磨砂玻璃。');
    });
    void refresh();
    window.addEventListener('focus', refresh);
    return () => window.removeEventListener('focus', refresh);
  }, []);

  useEffect(() => {
    if (!nativeHost || !bar.current) return;
    let active = true;
    let frame = 0;
    const update = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        if (!bar.current) return;
        updateNativeMaterial(bar.current, appearance, scene)
          .then(() => {
            if (active) {
              setNativeMaterial(true);
              document.documentElement.dataset.nativeBackdrop = 'true';
            }
          })
          .catch(() => {
            if (active) {
              setNativeMaterial(false);
              document.documentElement.dataset.nativeBackdrop = 'false';
            }
          });
      });
    };
    const observer = new ResizeObserver(update);
    observer.observe(bar.current);
    window.addEventListener('resize', update);
    window.addEventListener('scroll', update, { capture: true, passive: true });
    update();
    return () => {
      active = false;
      cancelAnimationFrame(frame);
      observer.disconnect();
      window.removeEventListener('resize', update);
      window.removeEventListener('scroll', update, { capture: true });
    };
  }, [appearance, scene, layoutKey]);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (composing.current || !text.trim() || submitting.current) return;
    // agent 追问时，主输入框直接作为“回复”通道，一键回给 agent。
    if (live?.answerMode && live.onSubmitAnswer) {
      submitting.current = true;setDispatching(true);
      try { await live.onSubmitAnswer(text);setText('');setNotice(null); } catch (error) { setNotice(String(error)); } finally { submitting.current = false;setDispatching(false); }
      return;
    }
    if (live?.busy) return;
    if(text.startsWith('/')){if(menuOpen&&options[menuIndex])await selectCommand(options[menuIndex]);else {setMenuDismissed(false);setNotice('请从斜杠菜单选择已注册的命令。');}return;}
    if(live){submitting.current=true;setDispatching(true);try{await live.onSubmit(text);setText('');setNotice(null);}catch(error){setNotice(String(error));}finally{submitting.current=false;setDispatching(false);}return;}
    // This UI milestone deliberately stops at a draft. No fake progress or file mutations.
    setNotice('这是界面预览，文件操作尚未启用。你的输入已保留。');
  };

  const closeSettings = () => { setOpen(false); };
  // 实时底栏展示 Finder 实际捕获的选区，用户提交前就能看到会处理哪些文件。
  const liveFiles=live?.files??[];
  const shownFiles=live?liveFiles:files;
  // 任务进行中：快捷按钮收起、输入框收成圆圈转圈，空出的横条按顺序掠过工具调用。
  const running=!!live?.busy&&!live?.answerMode;
  const activeTask=controller?.snapshot?.tasks.find(t=>t.id===controller.snapshot?.activeTaskId)??null;
  const ticked=useRef(0);
  const [ticker,setTicker]=useState<string[]>([]);
  useEffect(()=>{
    const actions=activeTask?.actions??[];
    if(!actions.length){ticked.current=0;setTicker([]);return;}
    if(actions.length<ticked.current)ticked.current=0;
    const fresh=actions.slice(ticked.current).map(action=>operationLabels[action.operation]??action.operation);
    ticked.current=actions.length;
    if(fresh.length)setTicker(current=>[...current,...fresh].slice(-5));
  },[activeTask?.id,activeTask?.actions.length]);

  return (
    <div className="agent-region" data-theme={appearance.theme}>
      <Reveal show={shownFiles.length>0||!!notice} edge="bottom" className="context-reveal">
        <div className="context-popover" role="status">
          {shownFiles.length > 0 && <div className="file-context"><File size={15} />
            <span title={shownFiles.join('\n')}>{shownFiles.length === 1 ? shownFiles[0].split('/').at(-1) : `已选择 ${shownFiles.length} 个文件：${shownFiles.slice(0,3).map(p=>p.split('/').at(-1)).join('、')}${shownFiles.length>3?'…':''}`}</span>
            <button className="small-icon" aria-label="清除所选文件" onClick={() => {if(live?.onClearFiles)void live.onClearFiles();else setFiles([]);}}><X size={14} /></button>
          </div>}
          {notice && <div className="notice"><ArrowUpRight size={15} /><span>{notice}</span>
            <button className="small-icon" aria-label="关闭提示" onClick={() => setNotice(null)}><X size={14} /></button>
          </div>}
        </div>
      </Reveal>
      <Reveal show={menuOpen} edge="bottom" className="slash-reveal"><SlashMenu options={options} index={menuIndex} busy={commandBusy} onSelect={o=>void selectCommand(o)} onHover={setMenuIndex} onSettings={()=>controller?void controller.openSettings('models'):setOpen(true)}/></Reveal>
      <div ref={bar} className="agent-bar" data-native-material={nativeMaterial} data-testid="agent-bar">
        <form onSubmit={submit} aria-label="Fleqi 文件操作" className="bar-controls" data-running={running?'true':undefined}>
          <button type="button" className="round-button add-button" aria-label="选择文件"
            title="选择文件" disabled={choosing||dispatching} onClick={async() => {if(live){setChoosing(true);try{await live.onChoose();}finally{setChoosing(false);}}else fileInput.current?.click();}}>{choosing?<LoaderCircle className="animate-spin"/>:<SvgImage name="plusCircleFill" size={22}/>}</button>
          {running&&<div className="tool-strip" aria-hidden="true">{ticker.map((label,index)=><span className="tool-chip" key={`${label}-${index}`}>{label}</span>)}{!ticker.length&&<span className="tool-chip idle">正在准备…</span>}</div>}
          <input ref={fileInput} className="visually-hidden" type="file" multiple tabIndex={-1} aria-label="添加文件"
            onChange={(event) => { setFiles(Array.from(event.target.files ?? [], (file) => file.name)); event.target.value = ''; input.current?.focus(); }} />
          <BorderBeam className="command-beam" size="md" colorVariant="mono" strength={0.7} theme={appearance.theme} active={!reducedMotion} borderRadius={24}>
          <input ref={input} className="command-input" aria-label="输入文件操作指令"
            placeholder={live?.answerMode?'回复 agent…':'向 Fleqi 描述要整理、改名或转换的文件…'} value={text} maxLength={4000}
            aria-controls={menuOpen?'slash-options':undefined} aria-expanded={menuOpen} aria-activedescendant={menuOpen&&options.length?`slash-option-${menuIndex}`:undefined} autoComplete="off" spellCheck={false} disabled={dispatching||running} onChange={(event) => { setText(event.target.value); setNotice(null);setMenuDismissed(false); }}
            onCompositionStart={() => { composing.current = true; }}
            onCompositionEnd={() => { composing.current = false; }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' && (event.nativeEvent.isComposing || composing.current || event.keyCode === 229)) event.preventDefault();
              if(menuOpen&&!composing.current&&!event.nativeEvent.isComposing){
                if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();setMenuIndex(i=>options.length?(i+(event.key==='ArrowDown'?1:-1)+options.length)%options.length:0);return;}
                if(event.key==='Enter'){event.preventDefault();if(options[menuIndex])void selectCommand(options[menuIndex]);return;}
                if(event.key==='Escape'){event.preventDefault();setMenuDismissed(true);return;}
              }
              if (event.key === 'Escape') { setNotice(null); input.current?.blur();if(live)void live.onHide(); }
            }} />
          {running&&<LoaderCircle className="running-spinner animate-spin" aria-hidden="true"/>}
          </BorderBeam>
          <button type="submit" className={`send-button ${live?.answerMode?'reply':''}`} aria-label={live?.answerMode?'回复 agent':'发送指令'}
            disabled={!text.trim()||dispatching||(live?.busy&&!live?.answerMode)} title={live?.answerMode?'回复 agent':'发送指令'}>{(dispatching||(live?.busy&&!live?.answerMode))?<LoaderCircle className="animate-spin"/>:live?.answerMode?<span className="reply-label">回复</span>:<SvgImage name="paperplaneCircle" size={28} className="send-glyph"/>}</button>
          <span className="bar-divider" aria-hidden="true" />
          <button ref={settings} type="button" className="round-button settings-button" aria-label={live?'设置':'外观设置'}
            aria-haspopup="dialog" aria-expanded={open} title="设置" onClick={() => live?void live.onSettings():setOpen(true)}><SettingsIcon /></button>
        </form>
      </div>
      <AppearancePanel open={open} appearance={appearance} support={support} onChange={onAppearance} onClose={closeSettings} />
    </div>
  );
}
