import {chromium} from '@playwright/test';
import {spawn} from 'node:child_process';
import {mkdir,rm} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const temporary=path.join(root,'.build/motion-recording');
const output=path.join(root,'docs/previews');
await mkdir(temporary,{recursive:true});await mkdir(output,{recursive:true});
const server=spawn('pnpm',['exec','vite','--host','127.0.0.1','--port','1422','--strictPort'],{cwd:root,stdio:['ignore','pipe','pipe'],detached:true});
let browser;
try {
  await new Promise((resolve,reject)=>{const timeout=setTimeout(()=>reject(new Error('Preview server did not start')),15000);server.stdout.on('data',data=>{if(data.toString().includes('1422')){clearTimeout(timeout);resolve();}});server.on('error',reject);server.on('exit',code=>{if(code)reject(new Error(`Preview server exited ${code}`));});});
  browser=await chromium.launch({channel:'chrome'});
  const context=await browser.newContext({viewport:{width:1080,height:760},recordVideo:{dir:temporary,size:{width:1080,height:760}},reducedMotion:'no-preference'});
  const page=await context.newPage();const video=page.video();
  await page.goto('http://127.0.0.1:1422');await page.getByRole('heading',{name:'概览',exact:true}).waitFor();
  await page.waitForTimeout(550);
  await page.getByRole('button',{name:'切换侧栏',exact:true}).click();await page.waitForTimeout(450);
  await page.getByRole('button',{name:'切换侧栏',exact:true}).click();await page.waitForTimeout(450);
  await page.locator('[data-slot="dropdown-menu-trigger"]').click();await page.waitForTimeout(450);await page.keyboard.press('Escape');await page.waitForTimeout(300);
  await page.getByRole('button',{name:'设置',exact:true}).click();await page.waitForTimeout(450);
  await page.getByRole('navigation',{name:'设置分类'}).getByRole('button',{name:'外观',exact:true}).click();await page.waitForTimeout(550);
  await page.getByRole('button',{name:'黑色',exact:true}).click();await page.waitForTimeout(450);
  await page.getByRole('button',{name:'浅色',exact:true}).click();await page.waitForTimeout(450);
  await page.getByRole('navigation',{name:'设置分类'}).getByRole('button',{name:'模型与账号',exact:true}).click();await page.waitForTimeout(550);
  await page.getByRole('button',{name:'添加连接',exact:true}).first().click();await page.waitForTimeout(450);
  await page.getByRole('combobox',{name:'连接方式'}).click();await page.getByRole('option',{name:'OpenAI Responses',exact:true}).click();await page.waitForTimeout(900);
  await page.screenshot({path:path.join(output,'motion-model-form.png')});
  await page.keyboard.press('Escape');await page.waitForTimeout(450);await page.getByRole('button',{name:'关闭设置'}).click();await page.waitForTimeout(550);
  await page.goto('http://127.0.0.1:1422/?surface=bar');
  const input=page.getByRole('textbox',{name:'输入文件操作指令'});await input.fill('/');await page.waitForTimeout(650);
  await page.screenshot({path:path.join(output,'motion-slash.png')});
  await input.press('Escape');await page.waitForTimeout(500);
  await context.close();await video.saveAs(path.join(output,'motion-demo.webm'));
  console.log('Saved docs/previews/motion-demo.webm');
} finally {
  await browser?.close();
  try{process.kill(-server.pid,'SIGTERM');}catch{/* Server already stopped. */}
  await rm(temporary,{recursive:true,force:true});
}
