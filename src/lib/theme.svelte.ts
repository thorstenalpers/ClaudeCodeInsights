import { setMode, mode } from 'mode-watcher';
import { applyThemeChange } from './theme/apply-theme-change';

export const PRESETS = [
	{ id: 'default', label: 'Neutral' },
	{ id: 'claude', label: 'Claude' },
	{ id: 'cosmic', label: 'Cosmic' },
	{ id: 'supabase', label: 'Supabase' },
	{ id: 'graphite', label: 'Graphite' }
] as const;

export type PresetId = (typeof PRESETS)[number]['id'];
export type ThemeMode = 'light' | 'dark' | 'system';

export type BrandToken = 'primary' | 'accent';
export type BrandColors = Record<BrandToken, string | null>;
export type BrandSettings = { light: BrandColors; dark: BrandColors };

const STORAGE_KEY = 'claudeadmin.theme.preset';
const BRAND_KEY = 'claudeadmin.theme.brand';
const BRAND_STYLE_ID = 'brand-overrides';

const EMPTY_BRAND: BrandSettings = {
	light: { primary: null, accent: null },
	dark: { primary: null, accent: null }
};

function readStoredPreset(): PresetId {
	if (typeof localStorage === 'undefined') return 'default';
	const stored = localStorage.getItem(STORAGE_KEY);
	return PRESETS.some((p) => p.id === stored) ? (stored as PresetId) : 'default';
}

function isHexColor(value: unknown): value is string {
	return typeof value === 'string' && /^#[0-9a-fA-F]{6}$/.test(value);
}

function readStoredBrand(): BrandSettings {
	if (typeof localStorage === 'undefined') return structuredClone(EMPTY_BRAND);
	try {
		const raw: unknown = JSON.parse(localStorage.getItem(BRAND_KEY) ?? 'null');
		const result = structuredClone(EMPTY_BRAND);
		for (const mode of ['light', 'dark'] as const) {
			for (const token of ['primary', 'accent'] as const) {
				const value = (raw as Record<string, Record<string, unknown>> | null)?.[mode]?.[token];
				if (isHexColor(value)) result[mode][token] = value;
			}
		}
		return result;
	} catch {
		return structuredClone(EMPTY_BRAND);
	}
}

/** White or near-black, whichever stays readable on the given colour. */
function foregroundFor(hex: string): string {
	const channel = (offset: number) => {
		const value = parseInt(hex.slice(offset, offset + 2), 16) / 255;
		return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
	};
	const luminance = 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
	return luminance > 0.4 ? '#1f1f1f' : '#ffffff';
}

function brandDeclarations(colors: BrandColors): string {
	const lines: string[] = [];
	if (colors.primary) {
		const fg = foregroundFor(colors.primary);
		lines.push(
			`--primary: ${colors.primary};`,
			`--primary-foreground: ${fg};`,
			`--ring: ${colors.primary};`,
			`--sidebar-primary: ${colors.primary};`,
			`--sidebar-primary-foreground: ${fg};`,
			`--sidebar-ring: ${colors.primary};`
		);
	}
	if (colors.accent) {
		const fg = foregroundFor(colors.accent);
		lines.push(
			`--accent: ${colors.accent};`,
			`--accent-foreground: ${fg};`,
			`--sidebar-accent: ${colors.accent};`,
			`--sidebar-accent-foreground: ${fg};`
		);
	}
	return lines.join('\n\t');
}

/** Tripled `:root` so the overrides outrank any `.dark.theme-x` preset rule. */
function brandCss(brand: BrandSettings): string {
	const light = brandDeclarations(brand.light);
	const dark = brandDeclarations(brand.dark);
	let css = '';
	if (light) css += `:root:root:root:not(.dark) {\n\t${light}\n}\n`;
	if (dark) css += `:root:root:root.dark {\n\t${dark}\n}\n`;
	return css;
}

class Theme {
	preset = $state<PresetId>(readStoredPreset());
	brand = $state<BrandSettings>(readStoredBrand());

	/** 'system' while following the OS, otherwise the explicit choice. */
	get mode(): ThemeMode {
		return mode.current ?? 'system';
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

	setBrandColor(themeMode: 'light' | 'dark', token: BrandToken, value: string | null): void {
		this.brand = {
			...this.brand,
			[themeMode]: { ...this.brand[themeMode], [token]: value }
		};
		localStorage.setItem(BRAND_KEY, JSON.stringify(this.brand));
		this.applyBrand();
	}

	resetBrand(): void {
		this.brand = structuredClone(EMPTY_BRAND);
		localStorage.removeItem(BRAND_KEY);
		this.applyBrand();
	}

	private applyBrand(): void {
		let style = document.getElementById(BRAND_STYLE_ID);
		if (!style) {
			style = document.createElement('style');
			style.id = BRAND_STYLE_ID;
			document.head.appendChild(style);
		}
		applyThemeChange(() => {
			style.textContent = brandCss(this.brand);
		});
	}

	/** Applies the stored preset and brand colours before the first paint. */
	init(): void {
		this.setPreset(this.preset);
		this.applyBrand();
	}
}

export const theme = new Theme();
