import {spawn} from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
function run(command,args){return new Promise((resolve,reject)=>{const child=spawn(command,args,{cwd:root,stdio:'inherit',shell:false});child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(new Error(`${command} exited ${code}`)));});}
// Resource writes must finish before Tauri walks the bundle tree.
await run('pnpm',['--dir','runtime/pi','install','--frozen-lockfile']);
await run(process.execPath,['scripts/prepare-runtime.mjs']);
await run('bash',['scripts/prepare-engines.sh']);
await run(process.execPath,['scripts/prepare-notices.mjs']);
await run('pnpm',['exec','tauri','build',...process.argv.slice(2)]);
