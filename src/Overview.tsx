import { Card } from './components/ui/card';
import { Badge } from './components/ui/badge';
import { Button } from './components/ui/button';
import { ActionButton } from './ui';
import { Interaction } from './Interaction';
import { AccentIcon,AssetIcon,PiIcon } from './icons';
import { GithubAccount } from './Account';
import { operationLabels,statusLabels,terminalStates,type FleqiController } from './backend';

const weekday=['一','二','三','四','五','六','日'];
function dayKey(value:number){const date=new Date(value);return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;}
function startOfWeek(value:Date){const date=new Date(value);date.setHours(0,0,0,0);const offset=(date.getDay()+6)%7;date.setDate(date.getDate()-offset);return date;}

/** 概览：把任务记录变成可读的使用情况面板（热力图 + 操作分布 + 关键指标）。 */
export function Overview({controller:c}:{controller:FleqiController}) {
  const d=c.snapshot!;
  const profile=d.profiles.find(p=>p.id===d.settings.defaultProfileId);
  const account=d.github?.account??null;
  const granted=d.permissions.filter(p=>p.status==='granted').length;
  const engines=d.engines.filter(e=>e.ready).length;
  const open=d.interactions;
  const tasks=d.tasks;
  const weeks=12;
  const today=new Date();
  const firstWeek=startOfWeek(new Date(today.getTime()-(weeks-1)*7*86400000));
  const counts=new Map<string,number>();
  let files=0;
  for(const task of tasks){files+=task.completed;counts.set(dayKey(task.createdAt),(counts.get(dayKey(task.createdAt))??0)+1);}
  const cells:number[][]=[];
  let peak=0;
  for(let week=0;week<weeks;week+=1){
    const column:number[]=[];
    for(let day=0;day<7;day+=1){
      const date=new Date(firstWeek.getTime()+(week*7+day)*86400000);
      const value=date>today?0:counts.get(dayKey(date.getTime()))??0;
      peak=Math.max(peak,value);column.push(value);
    }
    cells.push(column);
  }
  const totals=new Map<string,number>();
  for(const task of tasks)for(const action of task.actions)totals.set(action.operation,(totals.get(action.operation)??0)+1);
  const top=[...totals.entries()].sort((a,b)=>b[1]-a[1]).slice(0,5);
  const max=top[0]?.[1]??1;
  const finished=tasks.filter(t=>terminalStates.has(t.status)).length;
  const failed=tasks.filter(t=>t.status==='failed').length;
  const recent=tasks.slice(0,3);
  const level=(value:number)=>value===0?0:Math.min(4,Math.ceil(value/Math.max(1,Math.ceil(peak/4))));
  return <div className="overview-stack">
    <div className="overview-intro"><div><h2 className="text-2xl font-semibold tracking-tight">准备好，开始下一件事。</h2><p className="mt-2 text-xs text-muted-foreground">在 Finder 中选中文件，唤起底部栏并描述你的需求。</p></div><ActionButton disabled={!d.native||!d.settings.barEnabled} action={()=>c.run('bar_toggle',{show:true})}><AssetIcon name="sliderHalf" size={16}/>唤起底部栏</ActionButton></div>
    {open.length>0&&<div className="overview-open">{open.map(item=><Interaction key={item.id} value={item} controller={c}/>)}</div>}
    <div className="metric-grid">
      <Card className="metric-card"><span>已处理文件</span><strong>{files}</strong><small>来自 {tasks.length} 条任务记录</small></Card>
      <Card className="metric-card"><span>已完成任务</span><strong>{finished}</strong><small>{failed?`${failed} 条失败需要复核`:'没有失败记录'}</small></Card>
      <Card className="metric-card"><span>系统权限</span><strong>{d.native?`${granted}/${d.permissions.length}`:'—'}</strong><small>Finder 窗口、选区与文件访问</small></Card>
      <Card className="metric-card"><span>文件引擎</span><strong>{engines}/{d.engines.length||4}</strong><small>FFmpeg · qpdf · 基础文件</small></Card>
    </div>
    <div className="overview-grid">
      <Card className="chart-card heat-card">
        <div className="chart-head"><h3>任务热力图</h3><span>近 {weeks} 周</span></div>
        <div className="heatmap" role="img" aria-label={`近 ${weeks} 周的任务分布，最活跃的一天有 ${peak} 条任务`}>
          <div className="heat-days">{weekday.map((name,index)=><span key={name} className={index%2?'muted':''}>{name}</span>)}</div>
          <div className="heat-grid">{cells.map((column,index)=><div className="heat-week" key={index}>{column.map((value,day)=><span key={day} className={`heat-cell level-${level(value)}`} title={`${value} 条任务`}/>)}</div>)}</div>
        </div>
        <div className="heat-legend"><span>少</span>{[0,1,2,3,4].map(value=><span key={value} className={`heat-cell level-${value}`}/>) }<span>多</span></div>
      </Card>
      <Card className="chart-card">
        <div className="chart-head"><h3>操作分布</h3><span>累计 {[...totals.values()].reduce((a,b)=>a+b,0)} 次</span></div>
        {top.length?<div className="bar-list">{top.map(([operation,count])=><div className="bar-row" key={operation}><span className="bar-label">{operationLabels[operation]??operation}</span><span className="bar-track"><span className="bar-fill" style={{width:`${Math.max(6,Math.round(count/max*100))}%`}}/></span><span className="bar-value">{count}</span></div>)}</div>:<p className="chart-empty">还没有文件操作记录。</p>}
      </Card>
      <Card className="chart-card">
        <div className="chart-head"><h3>最近任务</h3><Button size="sm" variant="ghost" onClick={()=>c.navigate('tasks')}>全部</Button></div>
        {recent.length?<div className="recent-list">{recent.map(task=><button key={task.id} className="recent-row" onClick={()=>c.navigate('tasks')}><span className="min-w-0 flex-1"><strong className="truncate">{task.prompt}</strong><small>{new Date(task.createdAt).toLocaleString()} · {task.completed} 项操作</small></span><Badge variant="secondary">{statusLabels[task.status]??task.status}</Badge></button>)}</div>:<p className="chart-empty">从一个小任务开始，例如批量改名或转换格式。</p>}
      </Card>
      <Card className="chart-card">
        <div className="chart-head"><h3>账号与运行环境</h3></div>
        <div className="env-list">
          <GithubAccount controller={c} compact/>
          <div className="env-row"><AccentIcon name="customLink" size={15}/><span>模型连接</span><strong>{profile?.name??'尚未连接'}</strong></div>
          <div className="env-row"><PiIcon size={16}/><span>内置 PI</span><strong>{d.runtime.ready?d.runtime.pi:'未就绪'}</strong></div>
          <div className="env-row"><AccentIcon name="infoCircle" size={15}/><span>版本</span><strong>{d.runtime.app}</strong></div>
          {!account?.loggedIn&&<p className="account-hint">未登录 GitHub 时仍可正常处理文件任务。</p>}
        </div>
      </Card>
    </div>
  </div>;
}
