import type { Page } from '@playwright/test';

/** Disposable native-protocol fixture; no real account, keychain, Finder or model calls. */
export async function installHost(page: Page) {
  await page.addInitScript(() => {
    const callbacks = new Map<number, (event: unknown) => void>();
    const listeners = new Map<string, number[]>();
    let callbackId = 0;
    const context = { id: 'context-fixture', hostWindow: 'Finder', directory: '/fixture', files: ['/fixture/private-selection.png'], capturedAt: Date.now() };
    const profile = { id: 'profile-fixture', name: 'Fixture', protocol: 'openai-completions', baseUrl: 'https://example.test/v1', modelId: 'fixture-model', thinking: 'medium', authType: 'none', hasCredential: false, models: [], modelsFetchedAt: null, serviceTier: null };
    const snapshot: any = {
      settings: { revision: 1, appearance: { theme: 'light', material: 'frosted' }, launchAtLogin: false, barEnabled: true, activation: 'manual', shortcut: 'Control+Alt+Space', defaultProfileId: profile.id, outputDirectory: null, conflictPolicy: 'rename', bubbleSeconds: null },
      profiles: [profile], permissions: [], platform: { platform: 'macos', hostName: 'Finder', hostAttachment: true, tray: true, credentialStore: true, detail: 'fixture' },
      runtime: { ready: true, node: '24', pi: 'fixture', app: '0.0.1' }, engines: [], tasks: [], context, interactions: [], auth: null,
      github: { configured: true, mode: 'browser', account: null, flow: null }, update: { configured: false, repository: null, channel: 'stable', currentVersion: '0.0.1' }, activeTaskId: null, native: true,
    };
    const calls: { command: string; args: any }[] = [];
    const emit = (event = 'fleqi:changed') => (listeners.get(event) ?? []).forEach(id => callbacks.get(id)?.({ event, payload: null }));
    const finish = (status = 'completed', message = '已完成处理，原文件已保留。') => {
      if (snapshot.tasks[0]) Object.assign(snapshot.tasks[0], { status, message });
      snapshot.activeTaskId = null; snapshot.interactions = []; emit();
    };
    Object.assign(window, {
      isTauri: true,
      fixture: { snapshot, calls, emit, finish },
      __TAURI_INTERNALS__: {
        metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
        transformCallback: (callback: (event: unknown) => void) => { callbacks.set(++callbackId, callback); return callbackId; },
        unregisterCallback: (id: number) => callbacks.delete(id),
        invoke: async (command: string, args: any) => {
          calls.push({ command, args });
          if (command === 'plugin:event|listen') { listeners.set(args.event, [...listeners.get(args.event) ?? [], args.handler]); return args.handler; }
          if (command === 'plugin:event|unlisten') { listeners.set(args.event, (listeners.get(args.event) ?? []).filter(id => id !== args.eventId)); return; }
          if (command === 'app_snapshot') return structuredClone(snapshot);
          if (command === 'import_appearance') return;
          if (command === 'material_support') return { native: false, liquid: false, reduceTransparency: false };
          if (command === 'apply_material') throw new Error('fixture: use CSS material');
          if (command === 'bar_resize') return { spaceAbove: innerHeight - 56, spaceBelow: 0 };
          if (command === 'selection_peek') return snapshot.context.files;
          if (command === 'context_capture') return structuredClone(snapshot.context);
          if (command === 'update_check') return { available: false };
          if (command === 'task_submit') {
            const task = { id: 'task-fixture', requestId: args.input.requestId, prompt: args.input.prompt, context: structuredClone(context), profile, status: 'waiting_model', message: '', createdAt: Date.now(), updatedAt: Date.now(), sequence: 1, completed: 0, total: null, actions: [] };
            snapshot.tasks = [task]; snapshot.activeTaskId = task.id; emit(); return structuredClone(task);
          }
          if (command === 'interaction_answer') { snapshot.interactions = []; snapshot.tasks[0].status = 'waiting_model'; emit(); return; }
          if (command === 'task_cancel') { finish('cancelled', '任务已停止。'); return; }
          if (command === 'github_login_start') {
            snapshot.github.flow = { id: args.flowId, status: 'waiting', userCode: 'ABCD-1234', expiresAt: Date.now() + 900_000, message: null }; emit(); return structuredClone(snapshot.github.flow);
          }
          if (command === 'github_login_poll') return structuredClone(snapshot.github.flow);
          if (command === 'github_login_cancel') { snapshot.github.flow.status = 'cancelled'; snapshot.github.flow.userCode = ''; emit(); return; }
          if (command === 'github_logout') { snapshot.github.account = null; snapshot.github.flow = null; emit(); return; }
          throw new Error(`Unhandled fixture command: ${command}`);
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: (_event: string, id: number) => callbacks.delete(id) },
    });
  });
}
