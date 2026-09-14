import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Appearance, Material } from './appearance';

export const nativeHost = isTauri();
export interface MaterialSupport {
  native: boolean;
  liquid: boolean;
  reduceTransparency: boolean;
}

export async function getMaterialSupport(): Promise<MaterialSupport> {
  if (!nativeHost) return { native: false, liquid: false, reduceTransparency: false };
  return invoke<MaterialSupport>('material_support');
}

export async function updateNativeMaterial(
  element: HTMLElement,
  appearance: Appearance,
  scene: boolean,
): Promise<Material> {
  if (!nativeHost) return 'frosted';
  const rect = element.getBoundingClientRect();
  const radius = Number.parseFloat(getComputedStyle(element).getPropertyValue('--window-radius'));
  return invoke<Material>('apply_material', {
    request: {
      ...appearance,
      x: rect.x, y: rect.y, width: rect.width, height: rect.height,
      radius, scene,
    },
  });
}
