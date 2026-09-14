import { PageTransition } from './motion';
import { ArrowRight, LoaderCircle } from 'lucide-react';
import { SidebarInset,SidebarProvider,SidebarTrigger } from './components/ui/sidebar';
import { Breadcrumb,BreadcrumbItem,BreadcrumbList,BreadcrumbPage,BreadcrumbSeparator } from './components/ui/breadcrumb';
import { Card } from './components/ui/card';
import { Button } from './components/ui/button';
import { Separator } from './components/ui/separator';
import { AppSidebar } from './components/app-sidebar';
import { SettingsDialog } from './components/settings-dialog';
import { ActionButton,Notice } from './ui';
import { Overview } from './Overview';
import { UpdateDialog } from './Account';
import { Tasks,fileCapabilities } from './SettingsPages';
import { AssetIcon } from './icons';
import type { FleqiController } from './backend';
import './dashboard.css';
export function Dashboard({controller:c}:{controller:FleqiController}) {
  if(c.settingsSurface)return <SettingsDialog controller={c}/>;
  const name=c.page==='tasks'?'任务记录':c.page==='library'?'文件能力':'概览';
  return <div className="dashboard" data-theme={c.snapshot?.settings.appearance.theme??'light'}><SidebarProvider className="workspace-layout" style={{'--sidebar-width':'14rem'} as React.CSSProperties}><AppSidebar controller={c}/><SidebarInset className="workspace-main"><header className="workspace-header" data-tauri-drag-region><SidebarTrigger aria-label="切换侧栏"/><Separator orientation="vertical" className="mx-2 h-4" data-tauri-drag-region/><Breadcrumb data-tauri-drag-region><BreadcrumbList data-tauri-drag-region><BreadcrumbItem data-tauri-drag-region>工作区</BreadcrumbItem><BreadcrumbSeparator data-tauri-drag-region/><BreadcrumbItem data-tauri-drag-region><BreadcrumbPage data-tauri-drag-region><h1 className="text-sm font-medium" data-tauri-drag-region>{name}</h1></BreadcrumbPage></BreadcrumbItem></BreadcrumbList></Breadcrumb><ActionButton size="icon-sm" variant="ghost" className="ml-auto" aria-label="刷新状态" action={()=>c.refresh()}><AssetIcon name="clockRotate" size={14}/></ActionButton></header>
    {!c.snapshot?<div className="grid flex-1 place-content-center gap-3 text-center text-sm text-muted-foreground">{c.error?<><p role="alert">{c.error}</p><ActionButton variant="outline" action={()=>c.refresh()}>重新连接</ActionButton></>:<><LoaderCircle className="mx-auto animate-spin"/>正在加载工作区…</>}</div>:<PageTransition page={c.page} order={['overview','tasks','library']} className={c.page==='overview'?'overview-page':'content-page'}>{c.page==='overview'?<Overview controller={c}/>:c.page==='tasks'?<Tasks controller={c}/>:<div className="space-y-5"><div><h2 className="text-xl font-semibold">文件处理能力</h2><p className="mt-2 text-sm text-muted-foreground">从 Finder 选中文件后，以自然语言发起任务。</p></div><div className="grid grid-cols-2 gap-4">{fileCapabilities.map(([title,text])=><Card className="gap-3 p-5 shadow-none" key={title}><AssetIcon name="folder" size={22}/><h3 className="text-sm font-medium">{title}</h3><p className="text-xs leading-6 text-muted-foreground">{text}</p></Card>)}</div><Button variant="outline" onClick={()=>void c.openSettings('files')}>输出与文件处理设置<ArrowRight/></Button></div>}</PageTransition>}
    <footer className="workspace-footer"><span className="flex items-center gap-2"><span className="size-1.5 rounded-full bg-current"/>{c.snapshot?.native?'菜单栏常驻':'浏览器预览'}</span></footer>
  </SidebarInset></SidebarProvider><Notice controller={c}/><SettingsDialog controller={c}/><UpdateDialog controller={c}/></div>;
}
