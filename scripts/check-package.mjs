import {spawn} from 'node:child_process';
import {access,readdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const configuration=process.argv.includes('--release')?'release':'debug';
// --target aarch64-apple-darwin 会把产物放进 src-tauri/target/<triple>/<配置>/…，
// 不带 --target 时才是 src-tauri/target/<配置>/…。发版流程用前者，所以这里两种都找。
const targetFlag=process.argv.indexOf('--target');
const explicitTarget=targetFlag>=0?process.argv[targetFlag+1]:null;
const bundleRoot=path.join(root,'src-tauri','target');
const relative=`${configuration}/bundle/macos/Fleqi.app/Contents/Resources/runtime`;
const candidates=[];
if(explicitTarget)candidates.push(path.join(bundleRoot,explicitTarget,relative));
candidates.push(path.join(bundleRoot,relative));
try{
  for(const entry of await readdir(bundleRoot,{withFileTypes:true})){
    if(entry.isDirectory())candidates.push(path.join(bundleRoot,entry.name,relative));
  }
}catch{/* 还没有任何构建产物时，直接走下面报错。 */}
let resources=null;
for(const candidate of candidates){
  try{await access(path.join(candidate,'pi.tar.gz'));resources=candidate;break;}catch{/* 换下一个候选目录。 */}
}
if(!resources){
  throw new Error(`未找到打包后的运行时目录，请先构建应用（--bundles app）。已查找：\n${[...new Set(candidates)].map(item=>`  - ${item}`).join('\n')}`);
}
await new Promise((resolve,reject)=>{
  const child=spawn('cargo',['test','--manifest-path','src-tauri/Cargo.toml','--lib','runtime_package::tests'],{cwd:root,env:{...process.env,FLEQI_TEST_RESOURCES:resources},stdio:'inherit',shell:false});
  child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(new Error(`Package check failed (${code})`)));
});
