import {test,expect} from '@playwright/test';

test('overview fits the window; settings are a separate sidebar dialog',async({page})=>{
  await page.setViewportSize({width:1080,height:760});await page.goto('/');
  await expect(page.getByRole('heading',{name:'概览',exact:true})).toBeVisible();
  await expect(page.getByRole('button',{name:'唤起底部栏'})).toBeDisabled();
  const bounds=await page.evaluate(()=>({scroll:document.documentElement.scrollHeight,height:innerHeight,page:document.querySelector('.overview-page')!.getBoundingClientRect().bottom,footer:document.querySelector('.workspace-footer')!.getBoundingClientRect().bottom}));
  expect(bounds.scroll).toBe(bounds.height);expect(bounds.footer).toBeLessThanOrEqual(bounds.height);expect(bounds.page).toBeLessThan(bounds.footer);
  await page.screenshot({path:'docs/previews/dashboard-overview.png'});
  await page.getByRole('button',{name:'设置',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'Fleqi 设置'});await expect(dialog).toBeVisible();
  for(const name of ['通用','外观','模型与账号','权限与自检','文件处理','任务与诊断','关于与更新']){
    await dialog.getByRole('navigation',{name:'设置分类'}).getByRole('button',{name,exact:true}).click();
    await expect(dialog.getByRole('heading',{name,exact:true,level:1})).toBeVisible();
  }
  await dialog.getByRole('button',{name:'通用',exact:true}).click();
  await page.screenshot({path:'docs/previews/settings-general.png'});
  await dialog.getByRole('button',{name:'关闭设置'}).click();await expect(dialog).not.toBeVisible();
  await expect(page.getByRole('heading',{name:'概览',exact:true})).toBeVisible();
});
test('settings search finds a section by keyword',async({page})=>{
  await page.setViewportSize({width:1080,height:760});await page.goto('/#general');
  const dialog=page.getByRole('dialog',{name:'Fleqi 设置'});await expect(dialog).toBeVisible();
  const search=dialog.getByRole('textbox',{name:'搜索设置与功能'});
  await search.fill('更新');
  await expect(dialog.getByRole('button',{name:'关于与更新',exact:true})).toBeVisible();
  await expect(dialog.getByRole('button',{name:'文件处理',exact:true})).toHaveCount(0);
  await search.fill('没有任何匹配的设置');
  await expect(dialog.getByText('没有匹配的设置')).toBeVisible();
});
test('appearance and recorded shortcuts persist; native-only capabilities are unavailable in preview',async({page})=>{
  await page.goto('/#appearance');const dialog=page.getByRole('dialog',{name:'Fleqi 设置'});
  await dialog.getByRole('button',{name:'黑色',exact:true}).click();await expect(page.locator('.dashboard')).toHaveAttribute('data-theme','dark');await page.reload();await expect(page.locator('.dashboard')).toHaveAttribute('data-theme','dark');
  await expect(dialog.getByRole('radio',{name:'液态玻璃'})).toBeDisabled();
  await dialog.getByRole('button',{name:'通用',exact:true}).click();
  await expect(dialog.getByRole('switch',{name:'登录时启动'})).toBeDisabled();
  await expect(dialog.getByRole('combobox',{name:'唤起方式'})).toContainText('仅手动唤起');
  const recorder=dialog.getByRole('button',{name:'录入底部栏快捷键'});await recorder.click();await page.keyboard.press('Control+Alt+K');await expect(recorder).toContainText('K');await dialog.getByRole('button',{name:'保存',exact:true}).click();
  await dialog.getByRole('switch',{name:'启用底部快捷栏'}).click();await page.reload();await dialog.getByRole('button',{name:'通用',exact:true}).click();
  await expect(recorder).toContainText('K');await expect(dialog.getByRole('switch',{name:'启用底部快捷栏'})).toHaveAttribute('aria-checked','false');
  await page.screenshot({path:'docs/previews/settings-dark.png'});
});
test('Codex login fetches models without manual IDs; API input stays out of browser preview',async({page})=>{
  await page.goto('/#models');await page.getByRole('button',{name:'添加连接',exact:true}).first().click();
  const editor=page.getByRole('dialog',{name:'添加连接',exact:true});await expect(editor).toBeVisible();
  await expect(editor.getByRole('button',{name:'登录 Codex 并获取模型'})).toBeDisabled();
  await expect(editor.getByLabel('API 服务地址')).toHaveCount(0);await expect(editor.getByLabel('模型标识')).toHaveCount(0);
  await editor.getByRole('combobox',{name:'连接方式'}).click();await page.getByRole('option',{name:'OpenAI Responses',exact:true}).click();
  await expect(editor.getByLabel('API 服务地址')).toHaveValue('https://api.openai.com/v1');await expect(editor.getByLabel('API Key',{exact:true})).toBeDisabled();
  await expect(editor.getByRole('button',{name:'连接并获取模型'})).toBeDisabled();await page.keyboard.press('Escape');await expect(editor).not.toBeVisible();
  await expect(page.getByRole('dialog',{name:'Fleqi 设置'})).toBeVisible();
});
test('compact overview has no page scrolling and the usage panel owns its overflow',async({page})=>{
  await page.setViewportSize({width:800,height:560});await page.goto('/');
  const layout=await page.evaluate(()=>({h:innerHeight,doc:document.documentElement.scrollHeight,main:getComputedStyle(document.querySelector('.overview-page')!).overflow,stack:getComputedStyle(document.querySelector('.overview-stack')!).overflowY,metrics:document.querySelectorAll('.metric-card').length}));
  expect(layout.doc).toBe(layout.h);expect(layout.main).toBe('hidden');expect(layout.stack).toBe('auto');expect(layout.metrics).toBe(4);
  await expect(page.getByRole('button',{name:'设置',exact:true})).toBeInViewport();
  await page.screenshot({path:'docs/previews/dashboard-compact.png'});
});
