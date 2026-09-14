import { AnimatePresence, MotionConfig, motion, useIsPresent, useReducedMotion } from 'motion/react';
import { useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import tokens from './tokens.json';
export const curve = tokens.curve as [number, number, number, number];
export const duration = tokens.duration;
export const collapsedScale=tokens.collapsedScale;
export const transition = (kind:keyof typeof duration='layout', reduced=false) => ({ type:'tween' as const, ease:curve, duration:reduced?0:duration[kind] });
export function MotionProvider({children}:{children:ReactNode}) {
  const reduced=!!useReducedMotion();
  return <MotionConfig reducedMotion="user" transition={transition('layout',reduced)}>{children}</MotionConfig>;
}
// Observe natural content, not the animated wrapper, to avoid resize feedback loops.
export function AutoHeight({children,className='',initial=false}:{children:ReactNode;className?:string;initial?:boolean}) {
  const content=useRef<HTMLDivElement>(null);const [height,setHeight]=useState<number>();const reduced=!!useReducedMotion();
  useLayoutEffect(()=>{const element=content.current;if(!element)return;const measure=()=>setHeight(element.offsetHeight);measure();const observer=new ResizeObserver(measure);observer.observe(element);return()=>observer.disconnect();},[]);
  return <motion.div className={`motion-size ${className}`} initial={initial&&!reduced?{height:0}:false} animate={{height:height??'auto'}} transition={transition('layout',reduced)}><div ref={content} className="motion-size-content">{children}</div></motion.div>;
}
function RevealBody({children,edge,className}:{children:ReactNode;edge:'top'|'bottom';className:string}) {
  const reduced=!!useReducedMotion();const present=useIsPresent();
  return <motion.div data-motion="reveal" data-motion-state={present?'open':'closed'} inert={!present} aria-hidden={!present||undefined} className={`motion-reveal ${className}`} initial={reduced?false:{height:0,opacity:0,y:edge==='bottom'?8:-8}} animate={{height:'auto',opacity:1,y:0}} exit={{height:0,opacity:0,y:reduced?0:edge==='bottom'?8:-8}} transition={transition('layout',reduced)} style={{transformOrigin:edge==='bottom'?'50% 100%':'50% 0%'}}><AutoHeight>{children}</AutoHeight></motion.div>;
}
export function Reveal({show,children,edge='top',className=''}:{show:boolean;children:ReactNode;edge?:'top'|'bottom';className?:string}) {
  // Keep the last visible content intact while it returns to its origin.
  const last=useRef(children);if(show)last.current=children;
  return <AnimatePresence initial={false}>{show&&<RevealBody edge={edge} className={className}>{last.current}</RevealBody>}</AnimatePresence>;
}
function PageBody({children,className,direction}:{children:ReactNode;className:string;direction:number}) {
  const present=useIsPresent();const reduced=!!useReducedMotion();
  // 页面切换先虚化再交接：进入与离开都带高斯模糊过渡。
  return <motion.div className={className} data-motion="page" inert={!present} aria-hidden={!present||undefined} custom={direction} variants={{enter:(d:number)=>({opacity:0,x:reduced?0:12*d,filter:reduced?'blur(0px)':'blur(7px)'}),active:{opacity:1,x:0,filter:'blur(0px)'},leave:(d:number)=>({opacity:0,x:reduced?0:-12*d,filter:reduced?'blur(0px)':'blur(7px)'})}} initial={reduced?false:'enter'} animate="active" exit="leave" transition={transition('menu',reduced)}>{children}</motion.div>;
}
export function PageTransition({page,order,children,className}:{page:string;order:string[];children:ReactNode;className:string}) {
  const previous=useRef(page);const direction=order.indexOf(page)>=order.indexOf(previous.current)?1:-1;
  useLayoutEffect(()=>{previous.current=page;},[page]);
  return <AnimatePresence initial={false} mode="wait" custom={direction}><PageBody key={page} className={className} direction={direction}>{children}</PageBody></AnimatePresence>;
}
export { AnimatePresence, motion, useReducedMotion };
