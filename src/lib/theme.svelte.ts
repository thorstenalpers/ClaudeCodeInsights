import { setMode, mode, userPrefersMode } from 'mode-watcher';
import { applyThemeChange } from './theme/apply-theme-change';

export const PRESETS = [
	{ id: 'default', label: 'Default' },
	{ id: 'caffeine', label: 'Caffeine' },
	{ id: 'modern-minimal', label: 'Modern Minimal' },
	{ id: 'mono', label: 'Mono' },
	{ id: 'northern-lights', label: 'Northern Lights' },
	{ id: 'twitter', label: 'Twitter' },
	{ id: 'vercel', label: 'Vercel' }
] as const;

export type PresetId = (typeof PRESETS)[number]['id'];
export type ThemeMode = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'claudeadmin.theme.preset';

function readStoredPreset(): PresetId {
	if (typeof localStorage === 'undefined') return 'default';
	const stored = localStorage.getItem(STORAGE_KEY);
	return PRESETS.some((p) => p.id === stored) ? (stored as PresetId) : 'default';
}

class Theme {
	preset = $state<PresetId>(readStoredPreset());

	/**
	 * The setting, which is what the picker highlights.
	 *
	 * Deliberately not `mode.current`: that is the *resolved* mode and is only
	 * ever 'light' or 'dark', so reading it here left "System" impossible to
	 * select — picking it immediately highlighted whatever the OS had chosen.
	 */
	get mode(): ThemeMode {
		return userPrefersMode.current ?? 'system';
	}

	/**
	 * What is actually on screen, with 'system' already resolved.
	 *
	 * Reading `mode.current` is not just a query: mode-watcher applies the class
	 * to <html> inside that derived, so it only runs while something reads it.
	 */
	get resolvedMode(): 'light' | 'dark' {
		return mode.current === 'dark' ? 'dark' : 'light';
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

	/** Applies the stored preset and the mode before the first paint. */
	init(): void {
		this.setPreset(this.preset);
		// Reading the resolved mode is what puts the class on <html> — following
		// the OS at startup depends on this line, not on a component rendering.
		void this.resolvedMode;
	}
}

export const theme = new Theme();
