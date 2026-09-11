import type { Service } from './types';

export interface ViewPreferences { hidden: string[]; order: string[]; collapsed: string[] }
export interface ManagedService {
  key: string;
  name: string;
  cwd: string;
  buildCommand: string;
  runCommand: string;
  envText: string;
}
export const preferenceKey = 'devtopology.view.v1';
export const serviceConfigKey = 'devtopology.services.v1';
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

export function readManagedServices(): ManagedService[] {
  try {
    const value = JSON.parse(localStorage.getItem(serviceConfigKey) ?? '[]');
    return Array.isArray(value) ? value.filter((item): item is ManagedService =>
      item && typeof item.key === 'string' && typeof item.name === 'string' && typeof item.cwd === 'string'
    ) : [];
  } catch { return []; }
}

export function parseEnvironment(text: string): Record<string, string> {
  return Object.fromEntries(text.split('\n').map(line => line.trim()).filter(line => line && !line.startsWith('#'))
    .map(line => { const index = line.indexOf('='); return index > 0 ? [line.slice(0, index).trim(), line.slice(index + 1).trim()] : ['', '']; })
    .filter(([key]) => key));
}
