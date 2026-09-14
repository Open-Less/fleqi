import { currentOrigin } from './motion/origin';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef, useState } from 'react';
import { nativeHost } from './native';
import { readAppearance, type Appearance } from './appearance';

export interface SettingsSnapshot { revision:number; appearance:Appearance; launchAtLogin:boolean; barEnabled:boolean; activation:'manual'|'automatic'; shortcut:string; defaultProfileId:string|null; outputDirectory:string|null; conflictPolicy:'rename'|'ask'; bubbleSeconds:number|null }
export type Protocol='openai-completions'|'openai-responses'|'anthropic-messages'|'google-generative-ai'|'openai-codex-responses';
export interface ModelInfo { id:string; name:string; description:string; thinkingLevels:string[]; defaultThinking:string; serviceTiers:{id:string;name:string}[]; contextWindow:number; maxTokens:number; capabilitySource:string }
export interface ModelProfile { id:string; name:string; protocol:Protocol; baseUrl:string; modelId:string; thinking:string; authType:'api_key'|'oauth'|'none'; hasCredential:boolean; models:ModelInfo[]; modelsFetchedAt:number|null; serviceTier:string|null }
export interface ContextSnapshot { id:string; hostWindow:string; directory:string; files:string[]; capturedAt:number }
export interface Permission { id:string; name:string; purpose:string; status:string; detail:string; required:boolean }
export interface Interaction { id:string; kind:string; message:string; taskId:string|null; options:{id:string;label:string}[]|null }
/** 账号登录期间运行时下发的授权信息（授权链接或设备码），不含任何凭据。 */
export interface AuthState { type:string; url?:string; instructions?:string; message?:string; userCode?:string; verificationUri?:string }
/** GitHub 账号状态：令牌只保存在系统凭据存储，界面只拿展示用字段。 */
export interface GithubAccount { loggedIn:boolean; login?:string; name?:string; avatarUrl?:string; htmlUrl?:string }
export interface GithubState { configured:boolean; mode:'device'|'token'; account:GithubAccount|null }
export interface UpdateState { configured:boolean; repository:string|null; channel:'stable'|'beta'; currentVersion:string }
export interface UpdateInfo { available:boolean; version?:string; currentVersion?:string; notes?:string; date?:string }
export interface ActionRecord { id:string; operation:string; status:string; arguments:Record<string,unknown>; result:{outputs?:string[];message?:string;verified?:boolean}|null }
export interface TaskRecord { id:string; requestId:string; prompt:string; context:ContextSnapshot; profile:ModelProfile; status:string; message:string; createdAt:number; updatedAt:number; sequence:number; completed:number; total:number|null; actions:ActionRecord[] }
export interface Snapshot {
  settings:SettingsSnapshot; profiles:ModelProfile[]; permissions:Permission[];
  platform:{platform:string;hostName:string;hostAttachment:boolean;tray:boolean;credentialStore:boolean;detail:string};
  runtime:{ready:boolean;node:string;pi:string;app:string}; engines:{id:string;name:string;ready:boolean;version:string;formats:string}[];
  tasks:TaskRecord[]; context:ContextSnapshot|null; interactions:Interaction[]; auth:AuthState|null; github:GithubState; update:UpdateState; activeTaskId:string|null; native:boolean;
}
export const terminalStates=new Set(['completed','partial','failed','cancelled','needs_review']);
export const statusLabels:Record<string,string>={checking:'检查对象',waiting_model:'等待模型',awaiting_input:'待补充',awaiting_confirmation:'待确认',running:'执行中',verifying:'核验中',completed:'已完成',partial:'部分完成',failed:'失败',cancelled:'已取消',needs_review:'待核对'};
export const thinkingLabels:Record<string,string>={default:'模型默认',off:'关闭',minimal:'最低',low:'低',medium:'中',high:'高',xhigh:'更高',max:'最高',ultra:'超高'};
/** 文件操作的中文名：任务面板与底栏的工具条共用同一份。 */
export const operationLabels:Record<string,string>={
  rename:'改名',copy:'复制',move:'移动',create_directory:'新建文件夹',write_text:'写入文本',read_text:'读取文本',
  zip_create:'打包 ZIP',zip_extract:'解压',image_convert:'图片转换',media_convert:'音视频转换',media_trim:'裁剪',
  media_extract_audio:'提取音频',pdf_merge:'合并 PDF',pdf_split:'拆分 PDF',pdf_extract:'提取页面',pdf_rotate:'旋转 PDF',
  pdf_compress:'压缩 PDF',docx_create:'生成 DOCX',docx_read:'读取 DOCX',trash:'移到废纸篓',inspect:'检查',list:'列出',open:'打开',reveal:'显示位置',copy_path:'复制路径',
};
export const settingsPages=['general','appearance','models','permissions','files','tasks','about'];
const previewKey='fleqi.dashboard.preview.v1';
const defaults:SettingsSnapshot={revision:0,appearance:readAppearance(),launchAtLogin:false,barEnabled:true,activation:'manual',shortcut:'Control+Alt+Space',defaultProfileId:null,outputDirectory:null,conflictPolicy:'rename',bubbleSeconds:6.0};
function previewSnapshot():Snapshot {
  let settings=defaults;try{const saved=JSON.parse(localStorage.getItem(previewKey)??'null');if(saved?.appearance)settings={...defaults,...saved};}catch{/* Preview remains usable without storage. */}
  return {settings,profiles:[],permissions:[],platform:{platform:'browser',hostName:'Finder',hostAttachment:false,tray:false,credentialStore:false,detail:'浏览器预览；原生功能请在 Fleqi 应用中使用。'},runtime:{ready:false,node:'24.21.0',pi:'0.85.0',app:'0.0.1'},engines:[],tasks:[],context:null,interactions:[],auth:null,github:{configured:false,mode:'token',account:null},update:{configured:false,repository:null,channel:'stable',currentVersion:'0.0.1'},activeTaskId:null,native:false};
}
export async function backend<T>(command:string,args?:Record<string,unknown>):Promise<T> {
  if(nativeHost)return invoke<T>(command,args);
  if(command==='app_snapshot')return previewSnapshot() as T;
  if(command==='settings_save'){const settings={...(args?.settings as SettingsSnapshot),revision:(args?.settings as SettingsSnapshot).revision+1};localStorage.setItem(previewKey,JSON.stringify(settings));window.dispatchEvent(new Event('fleqi-preview-change'));return settings as T;}
  if(command==='shortcut_record')return undefined as T;
  throw new Error('请在 Fleqi 原生应用中使用此功能。');
}
type SettingsPatch=Partial<Omit<SettingsSnapshot,'appearance'>>&{appearance?:Partial<Appearance>};
export function useFleqi(){
  const initial=location.hash.slice(1)||'overview';
  const settingsSurface=new URLSearchParams(location.search).get('surface')==='settings';
  const [snapshot,setSnapshot]=useState<Snapshot|null>(null);const [error,setError]=useState<string|null>(null);const [notice,setNotice]=useState<string|null>(null);
  const [page,setPage]=useState(['overview','tasks','library'].includes(initial)?initial:'overview');
  const [settingsPage,setSettingsPage]=useState(settingsPages.includes(initial)?initial:'general');
  const [settingsOpen,setSettingsOpen]=useState(settingsSurface||settingsPages.includes(initial)&&!['tasks'].includes(initial));
  const current=useRef<Snapshot|null>(null);const saveQueue=useRef<Promise<unknown>>(Promise.resolve());
  const refresh=useCallback(async()=>{try{const value=await backend<Snapshot>('app_snapshot');current.current=value;setSnapshot(value);setError(null);return value;}catch(e){setError(String(e));return undefined;}},[]);
  const navigate=useCallback((value:string)=>{setPage(value);history.replaceState(null,'',`#${value}`);},[]);
  const openSettings=useCallback(async(value='general')=>{if(nativeHost&&!settingsSurface){try{await backend('open_settings',{page:value,origin:currentOrigin()});}catch(e){setNotice(String(e));}}else{setSettingsPage(value);setSettingsOpen(true);}},[settingsSurface]);
  const closeSettings=useCallback(async()=>{if(nativeHost&&settingsSurface)await backend('close_settings');else if(settingsSurface)location.assign('/');else setSettingsOpen(false);},[settingsSurface]);
  useEffect(()=>{
    let stopped=false;const cleanup:(()=>void)[]=[];let timer:ReturnType<typeof setTimeout>;
    const schedule=()=>{clearTimeout(timer);timer=setTimeout(()=>{void refresh();},80);};
    const init=async()=>{
      if(nativeHost){
        try{await invoke('import_appearance',{appearance:readAppearance()});}catch{/* Authoritative load reports persistence failures. */}
        for(const event of ['fleqi:changed','fleqi:task','fleqi:interaction']){const off=await listen(event,schedule);if(stopped)off();else cleanup.push(off);}
        const offNav=await listen<string>('fleqi:navigate',e=>{if(settingsSurface){setSettingsPage(e.payload);setSettingsOpen(true);}else navigate(e.payload);});
        const offNotice=await listen<string>('fleqi:notice',e=>setNotice(e.payload));
        const offAuth=await listen<{message?:string;instructions?:string}>('fleqi:auth',e=>{setNotice(e.payload.instructions??e.payload.message??'请在浏览器中完成授权。');schedule();});
        if(stopped){offNav();offNotice();offAuth();}else cleanup.push(offNav,offNotice,offAuth);
      }
      await refresh();
    };void init();window.addEventListener('focus',schedule);window.addEventListener('fleqi-preview-change',schedule);
    return()=>{stopped=true;clearTimeout(timer);cleanup.forEach(f=>f());window.removeEventListener('focus',schedule);window.removeEventListener('fleqi-preview-change',schedule);};
  },[navigate,refresh,settingsSurface]);
  useEffect(()=>{const dark=snapshot?.settings.appearance.theme==='dark';document.documentElement.classList.toggle('dark',dark);document.documentElement.dataset.theme=dark?'dark':'light';},[snapshot?.settings.appearance.theme]);
  // 通知自动消失：短提示 3.2 秒，长文本按长度放宽，最多 9 秒。用户也可以手动关闭。
  useEffect(()=>{
    if(!notice)return;
    const seconds=Math.min(9,Math.max(3.2,notice.length*0.09));
    const timer=setTimeout(()=>setNotice(value=>value===notice?null:value),seconds*1000);
    return()=>clearTimeout(timer);
  },[notice]);
  const run=useCallback(async<T,>(command:string,args?:Record<string,unknown>,success?:string):Promise<T|undefined>=>{setNotice(null);try{const result=await backend<T>(command,args);if(success)setNotice(success);await refresh();return result;}catch(e){setNotice(String(e));return undefined;}},[refresh]);
  const save=useCallback((patch:SettingsPatch)=>{const next=saveQueue.current.catch(()=>{}).then(async()=>{const value=current.current;if(!value)return false;const settings={...value.settings,...patch,appearance:{...value.settings.appearance,...patch.appearance}};try{const saved=await backend<SettingsSnapshot>('settings_save',{settings});const updated={...value,settings:saved};current.current=updated;setSnapshot(updated);setNotice('设置已保存');return true;}catch(e){setNotice(String(e));await refresh();return false;}});saveQueue.current=next;return next;},[refresh]);
  // 启动后自动检查更新；有更新时由界面弹出对话框，确认后再下载安装并重启。
  const [update,setUpdate]=useState<UpdateInfo|null>(null);
  const [updateProgress,setUpdateProgress]=useState<{downloaded:number;total:number|null;phase:string}|null>(null);
  const [updateBusy,setUpdateBusy]=useState(false);
  // 启动后的自动检查是静默的：更新通道还没发布过 Release（releases/latest 返回 404）
  // 时不该在界面上弹一条失败提示，用户主动点击“检查更新”时才报错。
  const checkUpdate=useCallback(async(options?:{silent?:boolean})=>{if(!nativeHost)return;try{const value=await backend<UpdateInfo>('update_check');setUpdate(value.available?value:null);return value;}catch(error){if(!options?.silent)setNotice(String(error));return undefined;}},[]);
  const installUpdate=useCallback(async()=>{setUpdateBusy(true);setUpdateProgress({downloaded:0,total:null,phase:'download'});try{await backend('update_install');}catch(e){setNotice(String(e));setUpdateBusy(false);setUpdateProgress(null);}return undefined;},[]);
  const dismissUpdate=useCallback(()=>{if(!updateBusy)setUpdate(null);},[updateBusy]);
  useEffect(()=>{
    if(!nativeHost)return;let off:(()=>void)|undefined;let stopped=false;
    void listen<{phase:string;downloaded?:number;total?:number}>('fleqi:update',e=>{const {phase,downloaded=0,total=null}=e.payload;setUpdateProgress({downloaded,total:total??null,phase});}).then(value=>{if(stopped)value();else off=value;});
    return()=>{stopped=true;off?.();};
  },[]);
  useEffect(()=>{const timer=setTimeout(()=>{void checkUpdate({silent:true});},1500);return()=>clearTimeout(timer);},[checkUpdate]);
  return {snapshot,error,notice,setNotice,page,navigate,refresh,run,save,settingsPage,setSettingsPage,settingsOpen,openSettings,closeSettings,settingsSurface,update,updateProgress,updateBusy,checkUpdate,installUpdate,dismissUpdate};
}
export type FleqiController=ReturnType<typeof useFleqi>;
