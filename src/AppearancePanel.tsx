import { Dialog,DialogContent,DialogTitle,DialogDescription } from './components/ui/dialog';
import { Check, Droplets, Layers2, Moon, Sun, X } from 'lucide-react';
import type { Appearance, Material, Theme } from './appearance';
import type { MaterialSupport } from './native';

interface Props {
  open: boolean;
  appearance: Appearance;
  support: MaterialSupport;
  onChange: (appearance: Partial<Appearance>) => void;
  onClose: () => void;
}

export function AppearancePanel({ open, appearance, support, onChange, onClose }: Props) {
  const setTheme = (theme: Theme) => onChange({ theme });
  const setMaterial = (material: Material) => onChange({ material });

  return (
    <Dialog open={open} onOpenChange={value=>{if(!value)onClose();}}><DialogContent className="appearance-panel" data-theme={appearance.theme} showCloseButton={false}>
      <DialogDescription className="sr-only">颜色与玻璃材质</DialogDescription>
      <div className="panel-heading">
        <div><span className="eyebrow">FLEQI</span><DialogTitle>外观</DialogTitle></div>
        <button className="small-icon" aria-label="关闭外观设置" onClick={onClose}><X size={18} /></button>
      </div>
      <fieldset>
        <legend>颜色</legend>
        <div className="theme-options">
          {(['light', 'dark'] as const).map((theme) => (
            <button key={theme} className={`theme-option ${appearance.theme === theme ? 'selected' : ''}`}
              aria-pressed={appearance.theme === theme} onClick={() => setTheme(theme)}>
              <span className={`mini-bar ${theme}`}><i /><i /><i /></span>
              <span>{theme === 'light' ? <Sun size={14} /> : <Moon size={14} />}
                {theme === 'light' ? '浅色' : '黑色'}
                {appearance.theme === theme && <Check className="option-check" size={15} />}
              </span>
            </button>
          ))}
        </div>
      </fieldset>
      <fieldset>
        <legend>玻璃材质</legend>
        <button className="material-option" aria-pressed={appearance.material === 'frosted' || !support.liquid}
          onClick={() => setMaterial('frosted')}>
          <Layers2 size={19} /><span><strong>磨砂玻璃</strong><small>柔和透光，清晰易读</small></span>
          {(appearance.material === 'frosted' || !support.liquid) && <Check size={17} />}
        </button>
        <button className="material-option" disabled={!support.liquid}
          aria-pressed={appearance.material === 'liquid' && support.liquid}
          onClick={() => setMaterial('liquid')}>
          <Droplets size={19} /><span><strong>液态玻璃 <em>macOS 26+</em></strong>
            <small>{support.liquid ? '原生光泽与动态折射' : '在支持的 macOS 应用中可选'}</small></span>
          {appearance.material === 'liquid' && support.liquid && <Check size={17} />}
        </button>
      </fieldset>
      {support.reduceTransparency && <p className="accessibility-note">已遵循系统“减少透明度”设置。</p>}
      <p className="panel-footnote">两种颜色共用相同的尺寸与圆角。</p>
    </DialogContent></Dialog>
  );
}
