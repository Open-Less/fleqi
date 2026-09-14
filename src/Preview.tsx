import { useState } from 'react';
import { ArrowUpRight, Moon, Sun } from 'lucide-react';
import { AgentBar } from './AgentBar';
import { readAppearance, saveAppearance, type Appearance, type Theme } from './appearance';
import { nativeHost } from './native';

const standalone = new URLSearchParams(location.search).get('surface') === 'bar';
document.documentElement.dataset.native = String(nativeHost);
document.documentElement.dataset.surface = standalone ? 'bar' : 'scene';

export function Preview() {
  const [appearance, setAppearance] = useState(readAppearance);
  const changeAppearance = (patch: Partial<Appearance>) => setAppearance((current) => {
    const next = { ...current, ...patch };
    saveAppearance(next);
    return next;
  });

  if (standalone) return <main className="standalone">
    <a className="back-to-scene" href="/">← 返回组合预览</a>
    <AgentBar appearance={appearance} onAppearance={changeAppearance} scene={false} />
  </main>;

  return (
    <main className="preview-scene">
      <div className="scene-background" aria-hidden="true" />
      <header className="preview-header" data-tauri-drag-region>
        <div className="wordmark" data-tauri-drag-region>Fleqi<span>底部控制栏</span></div>
        <div className="theme-switch" role="group" aria-label="预览颜色">
          {(['light', 'dark'] as const).map((theme: Theme) => (
            <button key={theme} onClick={() => changeAppearance({ theme })}
              aria-pressed={appearance.theme === theme} aria-label={`预览${theme === 'light' ? '浅色' : '黑色'}`}>
              {theme === 'light' ? <Sun size={15} /> : <Moon size={15} />}
              <span>{theme === 'light' ? '浅色' : '黑色'}</span>
            </button>
          ))}
        </div>
      </header>
      <section className="composition" aria-label="Finder 与 Fleqi 组合预览" data-testid="composition">
        <div className="finder-reference" aria-label="Finder 参考场景（静态图片）">
          <img src="/references/finder-light.png" alt="浅色 Finder 参考窗口，位于 Fleqi 控制栏上方" draggable={false} />
        </div>
        <AgentBar appearance={appearance} onAppearance={changeAppearance} scene />
      </section>
      <footer className="preview-footer">
        <span><i />交互预览<span className="footer-separator">·</span>尺寸以浅色参考图为准</span>
        <a href="?surface=bar">独立查看控制栏<ArrowUpRight size={14} /></a>
      </footer>
    </main>
  );
}
