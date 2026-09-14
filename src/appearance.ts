export type Theme = 'light' | 'dark';
export type Material = 'frosted' | 'liquid';
export interface Appearance { theme: Theme; material: Material }

export const APPEARANCE_KEY = 'fleqi.appearance.v1';
export const DEFAULT_APPEARANCE: Appearance = { theme: 'light', material: 'frosted' };

export function readAppearance(): Appearance {
  try {
    const saved = JSON.parse(localStorage.getItem(APPEARANCE_KEY) ?? 'null');
    return {
      theme: saved?.theme === 'dark' ? 'dark' : 'light',
      material: saved?.material === 'liquid' ? 'liquid' : 'frosted',
    };
  } catch {
    return DEFAULT_APPEARANCE;
  }
}

export function saveAppearance(appearance: Appearance) {
  try { localStorage.setItem(APPEARANCE_KEY, JSON.stringify(appearance)); }
  catch { /* Settings still work for the current session when storage is unavailable. */ }
}
