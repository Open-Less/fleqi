import { PageTransition } from '../motion';
// Fleqi settings, adapted from shadcn/ui sidebar-13.
import { useState } from 'react';
import { Search, X } from 'lucide-react';
import { Dialog,DialogContent,DialogDescription,DialogTitle } from './ui/dialog';
import { Sidebar,SidebarContent,SidebarGroup,SidebarGroupContent,SidebarMenu,SidebarMenuButton,SidebarMenuItem,SidebarProvider } from './ui/sidebar';
import { Breadcrumb,BreadcrumbItem,BreadcrumbList,BreadcrumbPage,BreadcrumbSeparator } from './ui/breadcrumb';
import { Button } from './ui/button';
import { Input } from './ui/input';
import type { FleqiController } from '../backend';
import { General,AppearanceSettings,Permissions,Files,Tasks,About } from '../SettingsPages';
import { Models } from '../Models';
import { Interaction } from '../Interaction';
import { Notice } from '../ui';
import { AccentIcon } from '../icons';
// 导航图标统一使用应用强调色（蓝 / 白 / 黑）；keywords 供设置搜索匹配。
const navigation=[
  ['general','通用','gearshape2','启动 登录 底部栏 唤起 快捷键 气泡 通知'],
  ['appearance','外观','paintpalette','主题 颜色 浅色 黑色 玻璃 材质 透明度'],
  ['models','模型与账号','customLink','模型 账号 Codex 登录 密钥 API 连接 GitHub'],
  ['permissions','权限与自检','shield','权限 辅助功能 自动化 完全磁盘访问 文件与文件夹 自检'],
  ['files','文件处理','folder','输出 目录 同名 引擎 FFmpeg PDF 图片 视频'],
  ['tasks','任务与诊断','clockRotate','任务 记录 诊断 日志 导出 历史'],
  ['about','关于与更新','infoCircle','关于 版本 更新 升级 GitHub 许可 内置 PI'],
] as const;
function SettingsLayout({controller:c}:{controller:FleqiController}) {
  const [query,setQuery]=useState('');
  const keyword=query.trim().toLowerCase();
  const visible=navigation.filter(([,name,,keywords])=>!keyword||name.toLowerCase().includes(keyword)||keywords.toLowerCase().includes(keyword));
  const page=navigation.find(([id])=>id===c.settingsPage)??navigation[0];
  return <SidebarProvider className="settings-layout" style={{'--sidebar-width':'12.5rem'} as React.CSSProperties}>
    <Sidebar collapsible="none" className="settings-sidebar"><SidebarContent><div className="settings-search"><Search size={13} aria-hidden="true"/><Input aria-label="搜索设置与功能" placeholder="搜索设置与功能" value={query} onChange={(event:React.ChangeEvent<HTMLInputElement>)=>setQuery(event.target.value)}/>{query&&<button type="button" aria-label="清除搜索" onClick={()=>setQuery('')}><X size={12}/></button>}</div><SidebarGroup><SidebarGroupContent><nav aria-label="设置分类"><SidebarMenu>{visible.map(([id,name,glyph])=><SidebarMenuItem key={id}><SidebarMenuButton isActive={page[0]===id} aria-current={page[0]===id?'page':undefined} onClick={()=>c.setSettingsPage(id)}><AccentIcon name={glyph} size={16}/><span>{name}</span></SidebarMenuButton></SidebarMenuItem>)}{!visible.length&&<p className="settings-search-empty">没有匹配的设置</p>}</SidebarMenu></nav></SidebarGroupContent></SidebarGroup></SidebarContent><div className="p-5 text-[11px] text-muted-foreground">Fleqi · {c.snapshot?.runtime.app??''}</div></Sidebar>
    <main className="settings-main"><header className="settings-header" data-tauri-drag-region={c.settingsSurface?true:undefined}><Breadcrumb><BreadcrumbList><BreadcrumbItem>设置</BreadcrumbItem><BreadcrumbSeparator/><BreadcrumbItem><BreadcrumbPage><h1 className="text-sm font-medium">{page[1]}</h1></BreadcrumbPage></BreadcrumbItem></BreadcrumbList></Breadcrumb><Button variant="ghost" size="icon-sm" aria-label="关闭设置" onClick={()=>void c.closeSettings()}><X/></Button></header><PageTransition page={page[0]} order={navigation.map(p=>p[0])} className="settings-scroll">
      {c.snapshot?<>{c.snapshot.interactions.filter(i=>!i.taskId).map(i=><Interaction key={i.id} value={i} controller={c}/>)}{page[0]==='general'&&<General controller={c}/>} {page[0]==='appearance'&&<AppearanceSettings controller={c}/>} {page[0]==='models'&&<Models controller={c}/>} {page[0]==='permissions'&&<Permissions controller={c}/>} {page[0]==='files'&&<Files controller={c}/>} {page[0]==='tasks'&&<Tasks controller={c}/>} {page[0]==='about'&&<About controller={c}/>}</>:<p className="p-5 text-sm text-muted-foreground">{c.error??'正在加载设置…'}</p>}
    </PageTransition></main>
  </SidebarProvider>;
}
export function SettingsDialog({controller:c}:{controller:FleqiController}) {
  if(c.settingsSurface)return <section className="settings-window" role="dialog" aria-label="Fleqi 设置"><SettingsLayout controller={c}/><Notice controller={c}/></section>;
  return <Dialog open={c.settingsOpen} onOpenChange={v=>{if(!v)void c.closeSettings();}}><DialogContent className="settings-dialog" showCloseButton={false}><DialogTitle className="sr-only">Fleqi 设置</DialogTitle><DialogDescription className="sr-only">通用、外观、模型与账号、权限、文件处理、任务和关于。</DialogDescription><SettingsLayout controller={c}/><Notice controller={c}/></DialogContent></Dialog>;
}
