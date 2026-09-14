// 提交信息与 PR 规范校验器。零依赖，CI 与本地钩子共用同一套规则。
//
// 用法：
//   node scripts/check-commits.mjs commit --message "feat(bar): 贴合 Finder 下边缘"
//   node scripts/check-commits.mjs commit --file .git/COMMIT_EDITMSG
//   node scripts/check-commits.mjs commit --range origin/beta..HEAD
//   node scripts/check-commits.mjs pr --title "..." --base beta --head feat/attached-bar
//
// 退出码 0 表示全部通过；1 表示存在违规，并打印可执行的修正说明。
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

// 与 AGENTS.md「Commit & Pull Request Guidelines」一致：类型固定为下列枚举，
// 主题用祈使句，中文或英文都可以。
const TYPES = ['feat', 'fix', 'docs', 'style', 'refactor', 'perf', 'test', 'build', 'ci', 'chore', 'revert'];
const HEADER_MAX = 100;
const SUBJECT_MAX = 72;
// 主干受保护：只有发版与热修复可以直接面向 main，功能开发一律先合入 beta。
const HEAD_PREFIXES = ['feat', 'fix', 'chore', 'docs', 'refactor', 'perf', 'test', 'ci', 'build', 'style', 'revert'];
const RELEASE_PREFIXES = ['release', 'hotfix'];
const HEADER_PATTERN = /^([a-z]+)(?:\(([^()]+)\))?(!)?: (.+)$/;

function fail(lines) {
  process.stderr.write(`${lines.join('\n')}\n`);
  process.exit(1);
}

function readMessage(args) {
  if (args.message !== undefined) return args.message;
  if (args.file !== undefined) return readFileSync(args.file, 'utf8');
  return null;
}

function checkHeader(header, { label }) {
  const problems = [];
  if (header.length > HEADER_MAX) problems.push(`标题 ${header.length} 字符，超过 ${HEADER_MAX} 上限`);
  if (/^(fixup|squash)!/.test(header)) problems.push('fixup!/squash! 提交必须先 squash 再合入');
  if (/^WIP\b/i.test(header)) problems.push('WIP 提交不能进入主干');
  const match = HEADER_PATTERN.exec(header);
  if (!match) {
    problems.push('缺少 “类型(范围): 主题” 结构，例如 fix(bar): 贴合 Finder 下边缘');
    return problems;
  }
  const [, type, scope, breaking, subject] = match;
  if (!TYPES.includes(type)) problems.push(`未知类型 “${type}”，可用：${TYPES.join('/')}`);
  if (scope !== undefined && scope.trim() === '') problems.push('范围括号不能为空，或整体省略');
  if (subject.trim().length < 4) problems.push('主题过短，说明这次改动做了什么');
  if (subject.length > SUBJECT_MAX) problems.push(`主题 ${subject.length} 字符，超过 ${SUBJECT_MAX} 上限`);
  if (/[.。;,；]$/.test(subject)) problems.push('主题结尾不要加句号或分号');
  if (/^(add|adds|added|fix|fixes|fixed|update|updates|updated|change|changes|changed)\b/i.test(subject)) {
    problems.push('主题用祈使句描述结果，例如 “保持底栏贴合” 而不是 “fixed 底栏贴合”');
  }
  if (problems.length) {
    process.stderr.write(`\n${label}\n  ${header}\n`);
  }
  void breaking;
  return problems;
}

// 权威来源是提交的完整信息：标题以外的空行、正文与 footer 不参与类型判定，
// 只解释可能存在的 BREAKING CHANGE 用法是否正确。
function checkMessage(raw, { label }) {
  const lines = raw.replace(/\r\n/g, '\n').split('\n');
  const header = (lines[0] ?? '').trim();
  const problems = checkHeader(header, { label });
  const body = lines.slice(1).join('\n').trim();
  const hasBreakingFooter = /^BREAKING CHANGE: /m.test(body);
  if (hasBreakingFooter && !/!/.test(header.split(':')[0])) {
    problems.push('存在 BREAKING CHANGE: footer 时，标题需要写 “!” 标记（例如 feat(api)!: ...）');
  }
  return problems;
}

function collectRange(range) {
  const output = execFileSync('git', ['log', '--no-merges', '--format=%H%x1f%B%x1e', range], { encoding: 'utf8' });
  return output
    .split('\x1e')
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => {
      const [sha, ...rest] = entry.split('\x1f');
      return { sha: sha.trim().slice(0, 8), message: rest.join('\x1f') };
    });
}

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith('--')) continue;
    const key = token.slice(2);
    args[key] = argv[index + 1] !== undefined && !argv[index + 1].startsWith('--') ? argv[++index] : true;
  }
  return args;
}

const [command, ...rest] = process.argv.slice(2);
const args = parseArgs(rest);
const problems = [];

if (command === 'commit') {
  const message = readMessage(args);
  if (message !== null) {
    if (!/^\s*(Merge |Revert )/.test(message)) problems.push(...checkMessage(message, { label: '提交信息不合规：' }));
  } else if (typeof args.range === 'string') {
    // 合入前统一看这一批提交，任何一条不合规都直接挡住。
    const commits = collectRange(args.range);
    if (!commits.length) process.stdout.write('没有需要校验的提交。\n');
    for (const entry of commits) {
      if (/^(Merge |Revert )/.test(entry.message)) continue;
      const found = checkMessage(entry.message, { label: `提交 ${entry.sha} 不合规：` });
      problems.push(...found);
    }
  } else {
    fail('用法：commit --message <文本> | --file <路径> | --range <base..head>');
  }
} else if (command === 'pr') {
  const title = typeof args.title === 'string' ? args.title.trim() : '';
  const base = typeof args.base === 'string' ? args.base : '';
  const head = typeof args.head === 'string' ? args.head.trim() : '';
  if (!title) problems.push('PR 标题为空；squash 合并会把标题写进主干，必须符合提交规范');
  else problems.push(...checkHeader(title, { label: 'PR 标题不合规：' }));
  if (head) {
    const prefix = head.split('/')[0];
    if (base === 'main' && !RELEASE_PREFIXES.includes(prefix)) {
      problems.push(`面向 main 的只有发版/热修复分支（${RELEASE_PREFIXES.join('、')}），当前是 ${head}；日常开发请以 beta 为目标分支`);
    }
    if (base === 'beta' && ![...HEAD_PREFIXES, ...RELEASE_PREFIXES].includes(prefix)) {
      problems.push(`分支名 ${head} 需要以 ${HEAD_PREFIXES.join('/')} 之一开头，例如 feat/attached-bar`);
    }
  }
} else {
  fail('用法：node scripts/check-commits.mjs <commit|pr> [选项]，详见文件头注释。');
}

if (problems.length) {
  process.stderr.write(`\n提交规范校验未通过：\n${problems.map((item) => `  - ${item}`).join('\n')}\n\n规则见 CONTRIBUTING.md。\n`);
  process.exit(1);
}
process.stdout.write('提交规范校验通过。\n');
