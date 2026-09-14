import { useEffect,useState } from 'react';
import { Button } from './components/ui/button';
import { Input } from './components/ui/input';
import { Dialog,DialogContent,DialogDescription,DialogTitle } from './components/ui/dialog';
import { AssetIcon } from './icons';
import { backend,type FleqiController } from './backend';

/** GitHub 账号：设备码登录，令牌只存系统凭据，界面只显示账号信息。 */
export function GithubAccount({controller:c,compact=false}:{controller:FleqiController;compact?:boolean}) {
  const github=c.snapshot?.github;
  const account=github?.account??null;
  const [flow,setFlow]=useState<{userCode:string;verificationUri:string;interval:number}|null>(null);
  const [busy,setBusy]=useState(false);
  const [message,setMessage]=useState<string|null>(null);
  useEffect(()=>{
    if(!flow)return;
    let stopped=false;let timer:ReturnType<typeof setTimeout>;
    const poll=async()=>{
      try{
        const result=await backend<{status:string;interval?:number}>('github_login_poll');
        if(stopped)return;
        if(result.status==='authorized'){setFlow(null);setMessage('已通过 GitHub 登录');await c.refresh();return;}
        timer=setTimeout(poll,Math.min(60,Math.max(2,result.interval??flow.interval))*1000);
      }catch(error){if(!stopped){setFlow(null);setMessage(String(error));}}
    };
    timer=setTimeout(poll,Math.min(60,Math.max(2,flow.interval))*1000);
    return()=>{stopped=true;clearTimeout(timer);};
  },[flow?.userCode]);
  const start=async()=>{if(busy)return;setBusy(true);setMessage(null);try{const started=await backend<{userCode:string;verificationUri:string;interval:number}>('github_login_start');setFlow(started);}catch(error){setMessage(String(error));}finally{setBusy(false);}};
  const logout=async()=>{setMessage(null);await c.run('github_logout',undefined,'已退出 GitHub 账号');};
  const copy=async()=>{if(!flow)return;try{await navigator.clipboard.writeText(flow.userCode);setMessage('设备码已复制');}catch{setMessage('复制失败，请手动输入设备码');}};
  if(!github?.configured){
    return <div className={compact?'account-block compact':'account-block'}>
      <div className="account-line"><AssetIcon name="customLink" size={16}/><span>GitHub 未配置</span></div>
      <p className="account-hint">创建 GitHub OAuth 应用并填写客户端 ID 后即可登录；步骤见 docs/GitHub_and_Updates.md。</p>
    </div>;
  }
  if(account?.loggedIn){
    return <div className={compact?'account-block compact':'account-block'}>
      <div className="account-line">{account.avatarUrl?<img className="account-avatar" src={account.avatarUrl} alt=""/>:<AssetIcon name="customLink" size={16}/>}<span className="truncate">{account.name||account.login}</span></div>
      <div className="account-actions"><span className="account-hint">已通过 GitHub 登录</span><Button size="sm" variant="ghost" onClick={()=>void logout()}>退出</Button></div>
    </div>;
  }
  return <div className={compact?'account-block compact':'account-block'}>
    <div className="account-line"><AssetIcon name="customLink" size={16}/><span>GitHub 账号</span></div>
    {flow?<div className="account-flow">
      <p className="account-hint">已在浏览器打开 GitHub 授权页，输入下面的设备码完成登录。</p>
      <div className="account-code"><code>{flow.userCode}</code><Button size="sm" variant="outline" onClick={()=>void copy()}>复制</Button></div>
      <div className="account-actions"><Button size="sm" variant="ghost" onClick={()=>void backend('github_status').then(()=>c.refresh())}>刷新状态</Button><Button size="sm" variant="ghost" onClick={()=>setFlow(null)}>取消</Button></div>
    </div>:<div className="account-actions"><Button size="sm" variant="outline" disabled={busy} onClick={()=>void start()}>{busy?'正在打开授权页…':'通过 GitHub 登录'}</Button>{message&&<span className="account-hint">{message}</span>}</div>}
    {message&&!flow&&<p className="account-hint">{message}</p>}
  </div>;
}

/** 自动更新：启动后检查，有新版本时弹窗说明并显示下载进度。 */
export function UpdateDialog({controller:c}:{controller:FleqiController}) {
  const update=c.update;const progress=c.updateProgress;
  const percent=progress?.total?Math.min(100,Math.round(progress.downloaded/progress.total*100)):null;
  const open=!!update||!!progress;
  if(!open)return null;
  const phase=progress?.phase??'prompt';
  return <Dialog open onOpenChange={value=>{if(!value)c.dismissUpdate();}}>
    <DialogContent className="update-dialog sm:max-w-md" showCloseButton={!c.updateBusy}>
      <DialogTitle>{phase==='prompt'?'发现新版本':'正在更新'}</DialogTitle>
      <DialogDescription>{phase==='prompt'?`新版本 ${update?.version??''} 已发布，更新会覆盖当前应用并自动重启。`:'下载完成后应用会自动重启，凭据与设置都会保留。'}</DialogDescription>
      {phase==='prompt'&&update?.notes&&<div className="update-notes">{update.notes}</div>}
      {phase!=='prompt'&&<div className="update-progress" role="progressbar" aria-valuenow={percent??undefined} aria-valuemin={0} aria-valuemax={100}>
        <span style={{width:percent!=null?`${percent}%`:'35%'}}/>
      </div>}
      {phase!=='prompt'&&<p className="account-hint">{phase==='download'?(percent!=null?`正在下载 ${percent}%`:'正在下载更新包…'):phase==='installing'?'正在安装并校验更新…':'即将重启完成更新…'}</p>}
      <div className="flex justify-end gap-2">
        {phase==='prompt'?<><Button variant="outline" onClick={()=>c.dismissUpdate()}>稍后</Button><Button disabled={c.updateBusy} onClick={()=>void c.installUpdate()}>立即更新</Button></>:<Button disabled variant="outline">请勿退出应用</Button>}
      </div>
    </DialogContent>
  </Dialog>;
}
