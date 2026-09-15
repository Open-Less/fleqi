import { Check, Circle, Pause } from 'lucide-react';
import { operationLabels, type TaskRecord } from './backend';

/** 只展示宿主记录的阶段和工具名称；文件选区与工具参数留在模型上下文中。 */
export function WorkProgress({ task, waiting = false }: { task?: TaskRecord | null; waiting?: boolean }) {
  const actions = (task?.actions ?? []).slice(-4);
  const phase = waiting ? '等待回复' : task?.status === 'running' ? '正在处理' : task?.status === 'verifying' ? '正在核验' : 'Thinking';
  const detail = waiting ? '在对话气泡中补充或确认' : task?.status === 'checking' ? '正在准备任务' : actions.length ? '正在整理结果' : '正在理解你的要求';
  return <div className="work-progress" role="status" aria-live="polite" aria-atomic="true">
    <span className="visually-hidden">{phase}。{actions.length ? operationLabels[actions.at(-1)!.operation] ?? '文件操作' : detail}</span>
    <div className="work-phase" aria-hidden="true"><span className="work-indicator">{waiting ? <Pause size={10} /> : <i />}</span><span>{phase}</span>{task?.total != null && task.total > 0 && <span className="work-count">{task.completed} / {task.total}</span>}</div>
    <div className="tool-strip" aria-hidden="true">
      {actions.length && !waiting ? <div className="tool-marquee">
        {[0, 1].map(copy => <div className="tool-batch" key={copy}>{actions.map(action => <span className="tool-chip" key={action.id} data-status={action.status}>
          {action.status === 'completed' ? <Check size={11} /> : <Circle size={9} />}<span>{operationLabels[action.operation] ?? '文件操作'}</span>
        </span>)}</div>)}
      </div> : <span className="work-detail">{detail}</span>}
    </div>
  </div>;
}
