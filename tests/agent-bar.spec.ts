import { expect, test, type Page } from '@playwright/test';

async function geometry(page: Page) {
  await expect(page.getByTestId('agent-bar')).toBeVisible();
  return page.evaluate(() => {
    const selectors = ['.composition', '.finder-reference', '.agent-bar', '.add-button', '.command-input', '.send-button', '.settings-button'];
    return selectors.map((selector) => {
      const element = document.querySelector(selector)!;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return { selector, x: rect.x, y: rect.y, width: rect.width, height: rect.height, radius: style.borderRadius };
    });
  });
}

test('light and black preserve every component dimension and all outer radii', async ({ page }) => {
  await page.goto('/?surface=preview');
  const light = await geometry(page);
  await page.screenshot({path:'docs/previews/control-bar-light.png'});
  await page.getByRole('button', { name: '预览黑色' }).click();
  await expect(page.locator('.agent-region')).toHaveAttribute('data-theme', 'dark');
  expect(await geometry(page)).toEqual(light);
  await page.screenshot({path:'docs/previews/control-bar-dark.png'});
  expect(light.find((item) => item.selector === '.agent-bar')?.height).toBe(68);
  expect(light.find((item) => item.selector === '.composition')?.radius).toBe('20px');
  expect(light.find((item) => item.selector === '.finder-reference')?.radius).toBe('20px');
  expect(light.find((item) => item.selector === '.agent-bar')?.radius).toBe('20px');
  await page.reload();
  await expect(page.locator('.agent-region')).toHaveAttribute('data-theme', 'dark');
});

test('appearance dialog is keyboard accessible and native glass is unavailable in browser', async ({ page }) => {
  await page.goto('/?surface=preview');
  await page.getByRole('button', { name: '外观设置' }).click();
  await expect(page.getByRole('dialog', { name: '外观' })).toBeVisible();
  await expect(page.getByRole('button', { name: /液态玻璃/ })).toBeDisabled();
  await page.getByRole('button', { name: '黑色', exact: true }).click();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await expect(page.getByRole('button', { name: '外观设置' })).toBeFocused();
  await expect(page.locator('.agent-region')).toHaveAttribute('data-theme', 'dark');
});

test('IME confirmation does not submit and preview never claims file execution', async ({ page }) => {
  await page.goto('/?surface=preview');
  const input = page.getByRole('textbox', { name: '输入文件操作指令' });
  await expect(page.getByRole('button', { name: '发送指令' })).toBeDisabled();
  await input.fill('把图片转为 JPG');
  await input.dispatchEvent('compositionstart', { data: '图片' });
  await input.press('Enter');
  await expect(page.getByRole('status')).toHaveCount(0);
  await input.dispatchEvent('compositionend', { data: '图片' });
  await input.press('Enter');
  await expect(page.getByRole('status')).toContainText('文件操作尚未启用');
  await expect(input).toHaveValue('把图片转为 JPG');
});

test('selecting files updates context without reading or uploading file contents', async ({ page }) => {
  await page.goto('/?surface=preview');
  await page.getByLabel('添加文件').setInputFiles([
    { name: '中文 图片.png', mimeType: 'image/png', buffer: Buffer.from('fixture') },
    { name: '第二张.jpg', mimeType: 'image/jpeg', buffer: Buffer.from('fixture') },
  ]);
  await expect(page.getByRole('status')).toContainText('已选择 2 个文件');
  await page.getByRole('button', { name: '清除所选文件' }).click();
  await expect(page.getByRole('status')).toHaveCount(0);
});

test('compact window preserves equal theme dimensions without horizontal overflow', async ({ page }) => {
  await page.setViewportSize({ width: 560, height: 620 });
  await page.goto('/?surface=preview');
  const light = await geometry(page);
  await page.getByRole('button', { name: '预览黑色' }).click();
  expect(await geometry(page)).toEqual(light);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(560);
});

test('input uses Beam and slash commands keep keyboard input safe',async({page})=>{
  await page.goto('/?surface=preview');const input=page.getByRole('textbox',{name:'输入文件操作指令'});
  await expect(page.locator('.command-beam[data-beam]')).toBeVisible();
  await input.fill('/');await expect(page.getByRole('listbox',{name:'斜杠命令'})).toBeVisible();
  await input.press('ArrowDown');await input.press('Enter');await expect(input).toHaveValue('/thinking ');
  await expect(page.getByText('没有可用选项。',{exact:false})).toBeVisible();
  await input.press('Escape');await expect(page.getByRole('listbox',{name:'斜杠命令'})).not.toBeVisible();
  await input.fill('/');await input.dispatchEvent('compositionstart');await input.press('Enter');await expect(input).toHaveValue('/');await input.dispatchEvent('compositionend');
  await page.emulateMedia({reducedMotion:'reduce'});await expect(page.locator('.command-beam')).not.toHaveAttribute('data-active','');
});


test('live bar opens the full settings dialog in browser preview',async({page})=>{
  await page.goto('/?surface=bar');await page.getByRole('button',{name:'设置',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'Fleqi 设置'});await expect(dialog).toBeVisible();
  await expect(dialog.getByRole('button',{name:'模型与账号',exact:true})).toBeVisible();
  await dialog.getByRole('button',{name:'关闭设置'}).click();await expect(dialog).not.toBeVisible();
  await expect(page.getByRole('textbox',{name:'输入文件操作指令'})).toBeVisible();
  await page.goto('/?surface=settings');await page.getByRole('button',{name:'关闭设置'}).click();
  await expect(page.getByRole('heading',{name:'概览',exact:true})).toBeVisible();
});

test('the selected-file chip stays visible instead of being clipped by the reveal wrapper',async({page})=>{
  await page.goto('/?surface=preview');
  await page.getByLabel('添加文件').setInputFiles([{name:'图片.png',mimeType:'image/png',buffer:Buffer.from('fixture')}]);
  const popover=page.locator('.context-popover');
  await expect(popover).toBeVisible();
  const box=await popover.boundingBox();expect(box&&box.height>20);
  // 揭示容器必须放开裁剪，否则绝对定位浮层整体不可见。
  await expect(page.locator('.context-reveal')).toHaveCSS('overflow','visible');
  await expect(page.locator('.context-popover .small-icon')).toBeVisible();
});

test('running state folds the input into a spinner circle and sweeps tools across the strip',async({page})=>{
  await page.goto('/?surface=preview&demo=running');
  const bar=page.getByTestId('agent-bar');
  await expect(bar.locator('.bar-controls')).toHaveAttribute('data-running','true');
  for(const selector of ['.add-button','.send-button','.bar-divider']){
    await expect(bar.locator(selector)).toHaveCSS('opacity','0');
  }
  const beamBox=await bar.locator('.command-beam').boundingBox();
  expect(beamBox).not.toBeNull();
  expect(Math.abs(beamBox!.width-beamBox!.height)).toBeLessThanOrEqual(2);
  const spinner=bar.locator('.running-spinner');
  await expect(spinner).toBeVisible();
  await expect(spinner).toHaveCSS('animation-name','spin');
  const strip=bar.locator('.tool-strip');
  await expect(strip).toBeVisible();
  const stripBox=await strip.boundingBox();
  expect(stripBox!.x).toBeLessThan(beamBox!.x);
  await expect(strip.locator('.tool-chip')).toHaveCount(1);
  await expect(bar.getByRole('textbox',{name:'输入文件操作指令'})).toBeDisabled();
});
