import {test,expect,type Page} from '@playwright/test';

async function capture(page:Page,selector:string,milliseconds=650) {
  await page.evaluate(({selector,milliseconds})=>{
    const frames:{time:number;x:number;y:number;width:number;height:number;opacity:number;transform:string}[]=[];
    const started=performance.now();
    (window as any).__motionFrames=new Promise(resolve=>{
      const tick=()=>{const element=document.querySelector<HTMLElement>(selector);if(element){const r=element.getBoundingClientRect();const s=getComputedStyle(element);frames.push({time:performance.now()-started,x:r.x,y:r.y,width:r.width,height:r.height,opacity:Number(s.opacity),transform:s.transform});}if(performance.now()-started<milliseconds)requestAnimationFrame(tick);else resolve(frames);};requestAnimationFrame(tick);
    });
  },{selector,milliseconds});
}
async function frames(page:Page):Promise<{time:number;x:number;y:number;width:number;height:number;opacity:number;transform:string}[]> {return page.evaluate(()=>(window as any).__motionFrames);}

test('settings uses its trigger as the origin and completes exit before removal',async({page})=>{
  await page.setViewportSize({width:1080,height:760});await page.goto('/');
  const trigger=page.getByRole('button',{name:'设置',exact:true});await expect(trigger).toBeVisible();
  await capture(page,'.settings-dialog');await trigger.click();const entered=await frames(page);
  expect(entered.some(f=>f.opacity>0&&f.opacity<.98)).toBeTruthy();
  const final=entered.at(-1)!;const early=entered.find(f=>f.opacity>.02)!;
  expect(early.width).toBeLessThan(final.width);expect(early.x).toBeGreaterThan(final.x);
  await capture(page,'.settings-dialog');await page.getByRole('button',{name:'关闭设置'}).click();const left=await frames(page);
  expect(left.some(f=>f.opacity>0&&f.opacity<.98)).toBeTruthy();expect(left.at(-1)!.width).toBeLessThan(final.width);
  await expect(page.getByRole('dialog',{name:'Fleqi 设置'})).toHaveCount(0);await expect(trigger).toBeFocused();
});

test('sidebar width follows the shared curve and reverses without a layout jump',async({page})=>{
  await page.setViewportSize({width:1080,height:760});await page.goto('/');
  const rail=page.locator('[data-slot="sidebar-gap"]');const before=(await rail.boundingBox())!.width;
  await capture(page,'[data-slot="sidebar-gap"]');await page.getByRole('button',{name:'切换侧栏',exact:true}).click();const collapsed=await frames(page);
  const after=collapsed.at(-1)!.width;expect(after).toBeLessThan(before);
  expect(collapsed.filter(f=>f.width>after+1&&f.width<before-1).length).toBeGreaterThan(2);
  expect(await rail.evaluate(e=>getComputedStyle(e).transitionTimingFunction)).toBe('cubic-bezier(0.2, 0.8, 0.2, 1)');
  await page.getByRole('button',{name:'切换侧栏',exact:true}).click();await expect.poll(async()=>(await rail.boundingBox())!.width).toBeCloseTo(before,0);
  await expect(page.getByRole('button',{name:'设置',exact:true})).toBeInViewport();
});

test('model form changes height smoothly and keeps a single font system',async({page})=>{
  await page.setViewportSize({width:1080,height:850});await page.goto('/#models');await page.getByRole('button',{name:'添加连接',exact:true}).first().click();
  const editor=page.getByRole('dialog',{name:'添加连接',exact:true});await expect(editor).toBeVisible();
  const body=editor.locator('.model-body-size');const initial=(await body.boundingBox())!.height;
  await editor.getByRole('combobox',{name:'连接方式'}).click();await capture(page,'.model-body-size',1000);await page.getByRole('option',{name:'OpenAI Responses',exact:true}).click();const changed=await frames(page);
  expect(changed.at(-1)!.height).toBeGreaterThan(initial+15);
  expect(new Set(changed.map(f=>Math.round(f.height))).size).toBeGreaterThan(3);
  const fonts=await page.evaluate(()=>['body','[data-slot="dialog-title"]','input','[data-slot="select-trigger"]','[data-slot="sidebar-menu-button"]'].map(s=>getComputedStyle(document.querySelector(s)!).fontFamily));
  expect(new Set(fonts).size).toBe(1);expect(fonts[0]).not.toContain('Inter');
});

test('slash panel grows upward while the input bottom remains fixed',async({page})=>{
  await page.setViewportSize({width:1000,height:550});await page.goto('/?surface=bar');
  const input=page.getByRole('textbox',{name:'输入文件操作指令'});const before=(await input.boundingBox())!;
  await capture(page,'.slash-reveal');await input.fill('/');const opened=await frames(page);
  expect(new Set(opened.map(f=>Math.round(f.height))).size).toBeGreaterThan(3);
  const after=(await input.boundingBox())!;expect(after.y+after.height).toBeCloseTo(before.y+before.height,0);
  await capture(page,'.slash-reveal');await input.press('Escape');const closed=await frames(page);expect(closed.at(-1)!.height).toBeLessThan(opened.at(-1)!.height);
  await expect(page.locator('.slash-reveal')).toHaveCount(0);
});

test('quick repeated open/close leaves no blocked layer; reduced motion keeps final geometry',async({page})=>{
  await page.goto('/');const trigger=page.getByRole('button',{name:'设置',exact:true});
  await trigger.click({force:true});await page.keyboard.press('Escape');await expect(page.getByRole('dialog')).toHaveCount(0);
  await trigger.click();await page.getByRole('button',{name:'关闭设置'}).click();await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.emulateMedia({reducedMotion:'reduce'});await trigger.click();
  await expect(page.getByRole('dialog',{name:'Fleqi 设置'})).toBeVisible();
  await expect.poll(()=>page.locator('.settings-dialog').evaluate(e=>getComputedStyle(e).transform)).toBe('none');
  await page.getByRole('button',{name:'关闭设置'}).click();await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(trigger).toBeFocused();
});


test('anchored menu animates from the sidebar and returns with the same curve',async({page})=>{
  await page.setViewportSize({width:1080,height:760});await page.goto('/');
  await capture(page,'[data-slot="dropdown-menu-content"]');await page.locator('[data-slot="dropdown-menu-trigger"]').click();
  const opened=await frames(page);expect(opened.some(f=>f.opacity>0&&f.opacity<.99)).toBeTruthy();
  const menu=page.locator('[data-slot="dropdown-menu-content"]');await expect(menu).toHaveAttribute('data-side','right');
  expect(await menu.evaluate(e=>getComputedStyle(e).animationTimingFunction)).toBe('cubic-bezier(0.2, 0.8, 0.2, 1)');
  await capture(page,'[data-slot="dropdown-menu-content"]');await page.keyboard.press('Escape');const closed=await frames(page);
  expect(closed.some(f=>f.opacity>0&&f.opacity<.99)).toBeTruthy();await expect(menu).toHaveCount(0);
});

test('compact sidebar opens from the left and releases its modal layer on close',async({page})=>{
  await page.setViewportSize({width:760,height:620});await page.goto('/');
  await capture(page,'[data-motion="sheet"]');await page.getByRole('button',{name:'切换侧栏',exact:true}).click();const opened=await frames(page);
  expect(opened.some(f=>f.x<-2)).toBeTruthy();expect(opened.at(-1)!.x).toBeCloseTo(0,0);
  await capture(page,'[data-motion="sheet"]');await page.keyboard.press('Escape');const closed=await frames(page);
  expect(closed.some(f=>f.x<-2),JSON.stringify({first:closed[0],last:closed.at(-1),count:closed.length})).toBeTruthy();await expect(page.locator('[data-motion="sheet"]')).toHaveCount(0);
  // 设置入口在侧栏底部，窄窗口下侧栏是抽屉，需要重新展开后才能点击。
  await page.getByRole('button',{name:'切换侧栏',exact:true}).click();await page.getByRole('button',{name:'设置',exact:true}).click();await expect(page.getByRole('dialog',{name:'Fleqi 设置'})).toBeVisible();
});
