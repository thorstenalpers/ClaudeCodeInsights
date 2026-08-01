import { setMode, mode } from 'mode-watcher';
import { applyThemeChange } from './theme/apply-theme-change';

export const PRESETS = [
  { id: 'default', label: 'Neutral' },
  { id: 'claude', label: 'Claude' },
  { id: 'cosmic', label: 'Cosmic' },
  { id: 'supabase', label: 'Supabase' },
  { id: 'graphite', label: 'Graphite' },
] as const;

export type PresetId = (typeof PRESETS)[number]['id'];
export type ThemeMode = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'cua.theme.preset';

function readStoredPreset(): PresetId {
  if (typeof localStorage === 'undefined') return 'default';
  const stored = localStorage.getItem(STORAGE_KEY);
  return PRESETS.some((p) => p.id === stored) ? (stored as PresetId) : 'default';
}

class Theme {
  preset = $state<PresetId>(readStoredPreset());

  /** 'system' while following the OS, otherwise the explicit choice. */
  get mode(): ThemeMode {
    return (mode.current ?? 'system') as ThemeMode;
  }

  setPreset(id: PresetId): void {
    applyThemeChange(() => {
      const root = document.documentElement;
      for (const preset of PRESETS) {
        root.classList.toggle(`theme-${preset.id}`, preset.id === id && id !== 'default');
      }
    });
    this.preset = id;
    localStorage.setItem(STORAGE_KEY, id);
  }

  setMode(next: ThemeMode): void {
    applyThemeChange(() => setMode(next));
  }

  /** Applies the stored preset before the first paint. */
  init(): void {
    this.setPreset(this.preset);
  }
}

export const theme = new Theme();
