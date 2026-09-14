// 发版前置校验：tag、package.json 与 tauri.conf.json 的版本必须一致，
// 并且更新通道（stable/beta）由 tag 后缀唯一决定。
//
// 用法：node scripts/check-version.mjs v0.0.1-beta.1
// 输出（写入 $GITHUB_OUTPUT，非 CI 环境直接打印）：
//   version=0.0.1-beta.1
//   channel=beta
import { readFileSync, appendFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const tag = (process.argv[2] ?? process.env.GITHUB_REF_NAME ?? '').trim();
if (!tag) {
  process.stderr.write('用法：node scripts/check-version.mjs v0.0.1[-beta.N]\n');
  process.exit(1);
}
const version = tag.replace(/^v/, '');
const channel = /-beta(\.|$)/.test(version) ? 'beta' : 'stable';

const packageVersion = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8')).version;
const tauriVersion = JSON.parse(readFileSync(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8')).version;

const problems = [];
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/.test(version)) problems.push(`tag ${tag} 不是 v<语义化版本> 形式`);
if (packageVersion !== version) problems.push(`package.json 版本 ${packageVersion} 与 tag ${version} 不一致`);
if (tauriVersion !== version) problems.push(`src-tauri/tauri.conf.json 版本 ${tauriVersion} 与 tag ${version} 不一致`);
if (problems.length) {
  process.stderr.write(`发版校验未通过：\n${problems.map((item) => `  - ${item}`).join('\n')}\n`);
  process.exit(1);
}

const output = [`version=${version}`, `channel=${channel}`, `prerelease=${channel === 'beta'}`].join('\n');
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `${output}\n`);
else process.stdout.write(`${output}\n`);
