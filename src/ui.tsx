import { motion,AnimatePresence,transition,useReducedMotion,AutoHeight } from './motion';
import { useState, type ComponentProps, type ReactNode } from 'react';
import { LoaderCircle, X } from 'lucide-react';
import { AssetIcon } from './icons';
import { Button } from './components/ui/button';
import { Card } from './components/ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './components/ui/select';
import type { FleqiController } from './backend';
const MotionButton=motion.create(Button);
export function ActionButton({action,children,disabled,...props}:Omit<ComponentProps<typeof MotionButton>,'onClick'|'children'> & {action:()=>Promise<unknown>;children:ReactNode}) {
  const [busy,setBusy]=useState(false);const reduced=!!useReducedMotion();
  return <MotionButton layout="size" transition={transition('feedback',reduced)} type="button" disabled={disabled||busy} aria-busy={busy} {...props} onClick={async()=>{if(busy)return;setBusy(true);try{await action();}finally{setBusy(false);}}}><AnimatePresence initial={false}>{busy&&<motion.span className="inline-flex overflow-hidden" initial={{width:0,opacity:0}} animate={{width:16,opacity:1}} exit={{width:0,opacity:0}} transition={transition('feedback',reduced)}><LoaderCircle className="size-4 animate-spin"/></motion.span>}</AnimatePresence><motion.span layout="position" className="inline-flex items-center gap-2">{children}</motion.span></MotionButton>;
}
export function Choice({label,value,onChange,options,disabled=false}:{label:string;value:string;onChange:(v:string)=>void;options:{value:string;label:string}[];disabled?:boolean}) {
  return <Select value={value} onValueChange={onChange} disabled={disabled}><SelectTrigger aria-label={label} className="min-w-32 max-w-full"><SelectValue placeholder="请选择"/></SelectTrigger><SelectContent>{options.map(o=><SelectItem key={o.value} value={o.value}>{o.label}</SelectItem>)}</SelectContent></Select>;
}
export function SettingRow({title,description,glyph,children}:{title:string;description?:string;glyph?:ReactNode;children?:ReactNode}) {return <div className="setting-row"><div className="min-w-0 flex-1"><h3 className="flex items-center gap-2.5 text-sm font-medium">{glyph}{title}</h3>{description&&<p className="mt-1.5 text-xs leading-5 text-muted-foreground">{description}</p>}</div><div className="setting-control">{children}</div></div>;}
export function SettingsGroup({children,title}:{children:ReactNode;title?:string}) {return <section>{title&&<h2 className="mb-3 text-sm font-medium">{title}</h2>}<Card className="gap-0 overflow-hidden py-0 shadow-none"><AutoHeight>{children}</AutoHeight></Card></section>;}
export function EmptyState({icon,children,title,description}:{icon:ReactNode;children?:ReactNode;title:string;description:string}){return <div className="empty-state"><span className="mb-4 text-muted-foreground">{icon}</span><h3 className="text-sm font-medium">{title}</h3><p className="mt-2 max-w-72 text-xs leading-6 text-muted-foreground">{description}</p>{children&&<div className="mt-5">{children}</div>}</div>;}
export function Notice({controller:c}:{controller:FleqiController}) {
  const reduced=!!useReducedMotion();
  return <AnimatePresence>{c.notice&&<motion.div role="status" className="feedback-toast" data-motion="notice" layout="size" initial={{opacity:0,y:reduced?0:12,scale:reduced?1:.98}} animate={{opacity:1,y:0,scale:1}} exit={{opacity:0,y:reduced?0:12,scale:reduced?1:.98}} transition={transition('menu',reduced)}><AssetIcon name="infoCircle" size={16}/><motion.span layout="position">{c.notice}</motion.span><Button size="icon-xs" variant="ghost" aria-label="关闭通知" onClick={()=>c.setNotice(null)}><X/></Button></motion.div>}</AnimatePresence>;
}
