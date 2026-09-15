import { expect, test, type Page } from '@playwright/test';
import { installHost } from './fixtures/host';

async function settleConversation(page: Page) {
  await expect.poll(() => page.locator('.conversation-reveal').evaluate(element => {
    const content = element.querySelector<HTMLElement>('.conversation-scroll')!;
    return Math.abs(element.getBoundingClientRect().height - content.offsetHeight) < 1;
  })).toBe(true);
}

test.beforeEach(async ({ page }) => { await installHost(page); });

test('the composer shrinks toward a fixed right edge; real tool activity moves right in a bounded strip', async ({ page }) => {
  await page.setViewportSize({ width: 900, height: 520 });
  await page.goto('/?surface=bar');
  const input = page.locator('.command-input');
  const bar = page.getByTestId('agent-bar');
  const initial = (await input.boundingBox())!;
  const initialBar = (await bar.boundingBox())!;
  await input.fill('把图片转换为 JPG');
  await page.evaluate(() => {
    (window as any).widths = [];
    const start = performance.now();
    const sample = () => { const r = document.querySelector('.command-beam')!.getBoundingClientRect(); (window as any).widths.push({ width: r.width, right: r.right }); if (performance.now() - start < 1000) requestAnimationFrame(sample); };
    requestAnimationFrame(sample);
  });
  await input.press('Enter');
  await expect(bar.locator('.running-control')).toBeVisible();
  await expect.poll(async () => Math.round((await page.locator('.command-beam').boundingBox())!.width)).toBe(34);
  const widths: { width: number; right: number }[] = await page.evaluate(() => (window as any).widths);
  expect(widths.some(r => r.width < initial.width - 2 && r.width > 36)).toBe(true);
  expect(widths.every(r => Math.abs(r.right - initial.x - initial.width) < 1)).toBe(true);
  expect((await bar.boundingBox())!.y).toBe(initialBar.y);
  await expect(page.locator('body')).not.toContainText('private-selection.png');
  await expect(page.locator('.add-badge')).toHaveCount(0);
  await expect(page.locator('.work-phase')).toHaveText('Thinking');
  await page.evaluate(() => {
    const f = (window as any).fixture;
    f.snapshot.tasks[0].status = 'running';
    f.snapshot.tasks[0].actions = Array.from({ length: 15 }, (_, i) => ({ id: `tool-${i}`, operation: i % 2 ? 'image_convert' : 'inspect', status: i === 14 ? 'running' : 'completed', arguments: { sources: ['/fixture/private-selection.png'] }, result: null }));
    f.emit();
  });
  await expect(page.locator('.tool-batch').first().locator('.tool-chip')).toHaveCount(4);
  const track = page.locator('.tool-marquee');
  await expect(track).toHaveCSS('animation-name', 'fleqi-tools-right');
  const movedRight = await track.evaluate(async element => {
    const x = () => new DOMMatrix(getComputedStyle(element).transform).m41;
    const start = x();
    await new Promise<void>(resolve => { let frames = 0; const tick = () => ++frames > 5 ? resolve() : requestAnimationFrame(tick); requestAnimationFrame(tick); });
    return x() > start;
  });
  expect(movedRight).toBe(true);
  await expect(page.locator('.work-progress')).toHaveCSS('background-color', 'rgba(255, 255, 255, 0.3)');
  expect((await bar.boundingBox())!.height).toBe(initialBar.height);
  await page.screenshot({ path: 'docs/previews/panel-working.png' });
  await page.getByRole('button', { name: '停止任务', exact: true }).click();
  await expect(page.locator('.chat-bubble.assistant')).toContainText('任务已停止');
  await expect(input).toBeEnabled();
  await page.getByRole('button', { name: '关闭对话' }).click();
  await expect(page.locator('.conversation-reveal')).toHaveCount(0);
  expect((await bar.boundingBox())!.y).toBe(initialBar.y);
});

test('questions stay inside chat bubbles, preserve IME input and never move the bar', async ({ page }) => {
  await page.setViewportSize({ width: 420, height: 340 });
  await page.goto('/?surface=bar');
  const bar = page.getByTestId('agent-bar');
  const before = (await bar.boundingBox())!;
  await page.locator('.command-input').fill('整理这些文档');
  await page.locator('.command-input').press('Enter');
  await page.evaluate(() => {
    const f = (window as any).fixture;
    f.snapshot.tasks[0].status = 'awaiting_input';
    f.snapshot.interactions = [{ id: 'question-fixture', taskId: 'task-fixture', kind: 'question', message: '文件名中的日期要使用哪一种格式？', options: null }]; f.emit();
  });
  const reply = page.getByRole('textbox', { name: '补充回答' });
  await expect(reply).toBeVisible();
  await reply.fill('保留日期，改成短横线分隔');
  await reply.dispatchEvent('compositionstart');
  await reply.dispatchEvent('keydown', { key: 'Enter', keyCode: 229, isComposing: true });
  expect(await page.evaluate(() => (window as any).fixture.calls.filter((c: any) => c.command === 'interaction_answer').length)).toBe(0);
  await reply.dispatchEvent('compositionend');
  await settleConversation(page);
  const bounds = await page.locator('.bar-overlays').boundingBox();
  expect(bounds!.y).toBeGreaterThanOrEqual(0);
  expect(bounds!.y + bounds!.height).toBeLessThan(before.y);
  expect((await bar.boundingBox())!.y).toBe(before.y);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(420);
  await page.screenshot({ path: 'docs/previews/panel-conversation.png' });
  await page.getByRole('button', { name: '回复', exact: true }).click();
  await expect(reply).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).fixture.calls.find((c: any) => c.command === 'interaction_answer').args.answer)).toBe('保留日期，改成短横线分隔');
  await page.evaluate(() => { const f = (window as any).fixture; f.snapshot.settings.appearance.theme = 'dark'; f.finish('completed', '日期格式已调整。'); });
  await expect(page.locator('.chat-bubble.assistant')).toContainText('日期格式已调整');
  expect((await bar.boundingBox())!.y).toBe(before.y);
  await settleConversation(page);
  await expect(page.locator('.command-beam')).toHaveCSS('width', `${Math.round((await page.locator('.composer-track').boundingBox())!.width)}px`);
  await page.screenshot({ path: 'docs/previews/panel-completed-dark.png' });
});

test('GitHub login has no PAT form, cancels the host flow and refreshes after browser authorization', async ({ page }) => {
  await page.goto('/?surface=settings#about');
  const account = page.locator('.account-block').last();
  await expect(page.getByLabel('GitHub 访问令牌')).toHaveCount(0);
  await account.getByRole('button', { name: '通过 GitHub 登录' }).click();
  await expect(account.locator('.account-code')).toContainText('ABCD-1234');
  await account.getByRole('button', { name: '取消登录' }).click();
  await expect(account.locator('.account-code')).toHaveCount(0);
  await account.getByRole('button', { name: '通过 GitHub 登录' }).click();
  await page.evaluate(() => { const f = (window as any).fixture; f.snapshot.github.flow.status = 'authorized'; f.snapshot.github.flow.userCode = ''; f.snapshot.github.account = { loggedIn: true, login: 'fixture-user', name: 'Fixture account' }; f.emit(); });
  await expect(account).toContainText('Fixture account');
  await expect(account.locator('.account-code')).toHaveCount(0);
  const commands: string[] = await page.evaluate(() => (window as any).fixture.calls.map((c: any) => c.command));
  expect(commands.filter(c => c === 'github_login_start')).toHaveLength(2);
  expect(commands.filter(c => c === 'github_login_cancel')).toHaveLength(1);
  expect(commands).not.toContain('github_login_token');
});
