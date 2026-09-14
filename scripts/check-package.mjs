import {spawn} from 'node:child_process';
import {access} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const configuration=process.argv.includes('--release')?'release':'debug';
const resources=path.join(root,`src-tauri/target/${configuration}/bundle/macos/Fleqi.app/Contents/Resources/runtime`);
await access(path.join(resources,'pi.tar.gz'));
await new Promise((resolve,reject)=>{
  const child=spawn('cargo',['test','--manifest-path','src-tauri/Cargo.toml','--lib','runtime_package::tests'],{cwd:root,env:{...process.env,FLEQI_TEST_RESOURCES:resources},stdio:'inherit',shell:false});
  child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(new Error(`Package check failed (${code})`)));
});
