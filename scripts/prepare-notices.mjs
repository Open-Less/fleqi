import {readFile,writeFile,readdir,mkdir,realpath} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const notices=[];const seen=new Set();
for(const store of ['node_modules/.pnpm','runtime/pi/node_modules/.pnpm']){
  for(const entry of await readdir(path.join(root,store),{withFileTypes:true})){
    if(!entry.isDirectory()||entry.name==='node_modules')continue;
    const modules=path.join(root,store,entry.name,'node_modules');
    let names;try{names=await readdir(modules,{withFileTypes:true});}catch{continue;}
    const packages=[];
    for(const name of names){
      if(name.name.startsWith('@')&&name.isDirectory())for(const child of await readdir(path.join(modules,name.name)))packages.push(path.join(modules,name.name,child));
      else packages.push(path.join(modules,name.name));
    }
    for(const folder of packages){
      let manifest;try{manifest=JSON.parse(await readFile(path.join(folder,'package.json'),'utf8'));}catch{continue;}
      const key=`${manifest.name}@${manifest.version}`;if(seen.has(key))continue;seen.add(key);
      let files;try{files=await readdir(await realpath(folder));}catch{continue;}
      const licenses=[];
      for(const name of files.filter(name=>/^(license|copying|notice)(\.|$|-)/i.test(name))){
        try{const content=await readFile(path.join(folder,name),'utf8');if(content.length<2_000_000)licenses.push(`${name}\n${content}`);}catch{/* License directories remain in the runtime dependency tree. */}
      }
      notices.push(`## ${key}\nDeclared license: ${typeof manifest.license==='string'?manifest.license:JSON.stringify(manifest.license??'not declared')}\n${licenses.length?licenses.join('\n\n'):'See the upstream package distribution for its license text.'}`);
    }
  }
}
const output=path.join(root,'src-tauri/runtime-resources/licenses');await mkdir(output,{recursive:true});
await writeFile(path.join(output,'JavaScript-NOTICES.txt'),`Fleqi bundled JavaScript component notices\n\n${notices.sort().join('\n\n---\n\n')}\n`);
await writeFile(path.join(output,'shadcn-LICENSE.txt'),await readFile(path.join(root,'docs/licenses/shadcn-LICENSE.txt'),'utf8'));
await writeFile(path.join(output,'BorderBeam-LICENSE.txt'),await readFile(path.join(root,'docs/licenses/BorderBeam-LICENSE.txt'),'utf8'));
console.log(`Prepared notices for ${notices.length} JavaScript packages.`);
