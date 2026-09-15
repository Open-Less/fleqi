import { useLayoutEffect, type RefObject } from 'react';
import { backend } from './backend';
import { nativeHost } from './native';

/** 网页和原生面板共用底边；只测浮层的实际高度，不再改变网页留白或展开方向。 */
export function useBarPanelSize(region: RefObject<HTMLElement | null>, ready: boolean) {
  useLayoutEffect(() => {
    const element = region.current;
    if (!nativeHost || !ready || !element) return;
    const bar = element.querySelector<HTMLElement>('.agent-bar');
    const overlays = element.querySelector<HTMLElement>('.bar-overlays');
    if (!bar || !overlays) return;
    let disposed = false;
    let frame = 0;
    let sending = false;
    let last = '';
    let next: { height: number; barHeight: number; barOffset: number } | null = null;
    const flush = async () => {
      if (sending || disposed) return;
      sending = true;
      try {
        while (next && !disposed) {
          const size = next;
          next = null;
          const key = JSON.stringify(size);
          if (last === key) continue;
          const result = await backend<{ spaceAbove?: number }>('bar_resize', size);
          last = key;
          if (!disposed && result.spaceAbove && result.spaceAbove > 24) {
            element.style.setProperty('--overlay-max-height', `${Math.floor(result.spaceAbove - 14)}px`);
          }
        }
      } catch {
        // 窗口暂时脱离 Finder 时，下次布局变化会重试；不把内部尺寸错误显示成对话。
        last = '';
      } finally { sending = false; }
    };
    const measure = () => {
      const barHeight = Math.ceil(bar.getBoundingClientRect().height);
      const overlayHeight = Math.ceil(overlays.getBoundingClientRect().height);
      const barOffset = overlayHeight > 0 ? overlayHeight + 14 : 0;
      next = { height: barHeight + barOffset, barHeight, barOffset };
      void flush();
    };
    const schedule = () => { cancelAnimationFrame(frame); frame = requestAnimationFrame(measure); };
    const observer = new ResizeObserver(schedule);
    observer.observe(bar);
    observer.observe(overlays);
    window.addEventListener('resize', schedule);
    measure();
    return () => { disposed = true; observer.disconnect(); cancelAnimationFrame(frame); window.removeEventListener('resize', schedule); };
  }, [region, ready]);
}
