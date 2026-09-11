import type { Service } from './types';

export interface ViewPreferences { hidden: string[]; order: string[]; collapsed: string[] }
export const preferenceKey = 'devtopology.view.v1';
export function serviceKey(service: Service): string {
  return JSON.stringify([service.project?.path ?? '', service.process, [...service.ports].sort((a,b) => a-b)]);
}
export function readPreferences(): ViewPreferences {
  try {
    const value = JSON.parse(localStorage.getItem(preferenceKey) ?? '{}');
    const strings = (v: unknown): string[] => Array.isArray(v) ? v.filter(x => typeof x === 'string') : [];
    return { hidden: strings(value.hidden), order: strings(value.order), collapsed: strings(value.collapsed) };
  } catch { return { hidden: [], order: [], collapsed: [] }; }
}
