import { useEffect,useRef,useState } from 'react';
import { Button } from './components/ui/button';
import { Dialog,DialogContent,DialogDescription,DialogTitle } from './components/ui/dialog';
import { AssetIcon } from './icons';
import { backend,type FleqiController,type GithubFlow } from './backend';

/** 用户在系统浏览器中登录 GitHub 并授权组织 App，凭据由宿主保存。 */
export function GithubAccount({controller:c,compact=false}:{controller:FleqiController;compact?:boolean}) {
  const github=c.snapshot?.github;
  const account=github?.account??null;
  const [flow,setFlow]=useState<GithubFlow|null>(null);
  const [busy,setBusy]=useState(false);
  const [message,setMessage]=useState<string|null>(null);
  const starting=useRef(false);
  const remote=github?.flow;
  useEffect(()=>{
    if(remote&&(remote.status==='starting'||remote.status==='waiting'))setFlow(remote);
  },[remote?.id,remote?.status,remote?.userCode]);
  useEffect(()=>{
    if(!flow)return;
    let stopped=false;let timer:ReturnType<typeof setTimeout>;
    const poll=async()=>{
      try{
        const current=await backend<GithubFlow>('github_login_poll',{flowId:flow.id});
        if(stopped)return;
        if(current.status==='authorized'){setFlow(null);setMessage(null);await c.refresh();return;}
        if(['failed','expired','cancelled'].includes(current.status)){setFlow(null);setMessage(current.message??(current.status==='cancelled'?'已取消登录':'登录未完成，请重试'));await c.refresh();return;}
        setFlow(current);timer=setTimeout(poll,1000);
      }catch(error){if(!stopped){setFlow(null);setMessage(String(error));}}
    };
    timer=setTimeout(poll,500);
    return()=>{stopped=true;clearTimeout(timer);};
  },[flow?.id,c.refresh]);
  const start=async()=>{
    if(starting.current||flow)return;
    starting.current=true;setBusy(true);setMessage(null);
    try{setFlow(await backend<GithubFlow>('github_login_start',{flowId:crypto.randomUUID()}));await c.refresh();}
    catch(error){setMessage(String(error));}
    finally{starting.current=false;setBusy(false);}
  };
  const cancel=async()=>{if(!flow)return;setBusy(true);try{await backend('github_login_cancel',{flowId:flow.id});setFlow(null);setMessage('已取消登录');await c.refresh();}catch(error){setMessage(String(error));}finally{setBusy(false);}};
  const logout=async()=>{setMessage(null);await c.run('github_logout',undefined,'已退出 GitHub 账号');};
  const copy=async()=>{if(!flow?.userCode)return;try{await navigator.clipboard.writeText(flow.userCode);setMessage('验证码已复制');}catch{setMessage('请在 GitHub 页面输入上方验证码');}};
  if(account?.loggedIn){
    return <div className={compact?'account-block compact':'account-block'}>
      <div className="account-line">{account.avatarUrl?<img className="account-avatar" src={account.avatarUrl} alt=""/>:<AssetIcon name="customLink" size={16}/>}<span className="truncate">{account.name||account.login}</span></div>
      <div className="account-actions"><span className="account-hint">已通过 GitHub 登录</span><Button size="sm" variant="ghost" onClick={()=>void logout()}>退出</Button></div>
    </div>;
  }
  return <div className={compact?'account-block compact':'account-block'}>
    <div className="account-line"><AssetIcon name="customLink" size={16}/><span>GitHub 账号</span></div>
    {flow?<div className="account-flow" aria-live="polite">
      <p className="account-hint">{flow.userCode?'在弹出的 GitHub 窗口输入验证码，登录并授权 Fleqi Desktop。完成后会自动返回。':'正在打开 GitHub 登录窗口…'}</p>
      {flow.userCode&&<div className="account-code"><code>{flow.userCode}</code><Button size="sm" variant="outline" onClick={()=>void copy()}>复制验证码</Button></div>}
      <Button size="sm" variant="ghost" disabled={busy} onClick={()=>void cancel()}>取消登录</Button>
    </div>:<>
      <p className="account-hint">使用 GitHub 账号登录，授权 Open-Less 的 Fleqi Desktop 应用。</p>
      <Button size="sm" variant="outline" disabled={busy||!github?.configured} onClick={()=>void start()}>{busy?'正在打开…':'通过 GitHub 登录'}</Button>
      {!github?.configured&&<p className="account-hint">此版本尚未启用 GitHub 登录。</p>}
    </>}
    {message&&<p className="account-hint" role="status">{message}</p>}
  </div>;
}

/** 自动更新：启动后检查，有新版本时弹窗说明并显示下载进度。 */
export function UpdateDialog({controller:c}:{controller:FleqiController}) {
  const update=c.update;const progress=c.updateProgress;
  const percent=progress?.total?Math.min(100,Math.round(progress.downloaded/progress.total*100)):null;
  const open=!!update||!!progress;
  if(!open)return null;
  const phase=progress?.phase??'prompt';
  const channel=c.snapshot?.update.channel??'stable';
  return <Dialog open onOpenChange={value=>{if(!value)c.dismissUpdate();}}>
    <DialogContent className="update-dialog sm:max-w-md" showCloseButton={!c.updateBusy}>
      <DialogTitle>{phase==='prompt'?'发现新版本':'正在更新'}</DialogTitle>
      <DialogDescription>{phase==='prompt'?`${channel==='beta'?'测试通道':'正式通道'}的新版本 ${update?.version??''} 已发布，更新会覆盖当前应用并自动重启。`:'下载完成后应用会自动重启，凭据与设置都会保留。'}</DialogDescription>
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
