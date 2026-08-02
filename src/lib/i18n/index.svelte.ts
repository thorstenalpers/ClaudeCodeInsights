/**
 * The active language, and the lookup every view uses.
 *
 * There is no i18n library behind this: five flat catalogs and a lookup are all
 * the app needs, and the English catalog doubles as the type, so a key that
 * exists nowhere is a compile error rather than a blank spot in the window.
 */
import { de } from './de';
import { en, type MessageKey, type Messages } from './en';
import { es } from './es';
import { fr } from './fr';
import { it } from './it';

export const LOCALES = [
	{ id: 'en', label: 'English' },
	{ id: 'de', label: 'Deutsch' },
	{ id: 'fr', label: 'Français' },
	{ id: 'es', label: 'Español' },
	{ id: 'it', label: 'Italiano' }
] as const;

export type Locale = (typeof LOCALES)[number]['id'];
/** 'system' follows the OS; anything else is the user's explicit choice. */
export type LocaleSetting = Locale | 'system';

const CATALOGS: Record<Locale, Messages> = { en, de, fr, es, it };
const STORAGE_KEY = 'claudeadmin.locale';

function isLocale(value: unknown): value is Locale {
	return LOCALES.some((entry) => entry.id === value);
}

/** The OS language, as the WebView reports it, narrowed to what we translate. */
function systemLocale(): Locale {
	if (typeof navigator === 'undefined') return 'en';
	for (const tag of navigator.languages ?? [navigator.language]) {
		const base = tag.split('-')[0]?.toLowerCase();
		if (isLocale(base)) return base;
	}
	return 'en';
}

function readStored(): LocaleSetting {
	if (typeof localStorage === 'undefined') return 'system';
	const stored = localStorage.getItem(STORAGE_KEY);
	return stored === 'system' || isLocale(stored) ? stored : 'system';
}

class I18n {
	setting = $state<LocaleSetting>(readStored());

	/** The language actually in use, with 'system' resolved. */
	get locale(): Locale {
		return this.setting === 'system' ? systemLocale() : this.setting;
	}

	/** A BCP 47 tag for Intl, which wants a region for sensible defaults. */
	get intlLocale(): string {
		return { en: 'en-US', de: 'de-DE', fr: 'fr-FR', es: 'es-ES', it: 'it-IT' }[this.locale];
	}

	set(next: LocaleSetting): void {
		this.setting = next;
		if (next === 'system') {
			localStorage.removeItem(STORAGE_KEY);
		} else {
			localStorage.setItem(STORAGE_KEY, next);
		}
		document.documentElement.lang = this.locale;
	}

	/** Looks a message up, filling `{placeholders}` from `params`. */
	t(key: MessageKey, params?: Record<string, string | number>): string {
		const message = CATALOGS[this.locale][key] ?? en[key];
		if (!params) return message;
		return message.replace(/\{(\w+)\}/g, (whole, name: string) =>
			name in params ? String(params[name]) : whole
		);
	}

	init(): void {
		document.documentElement.lang = this.locale;
	}
}

export const i18n = new I18n();

/** Shorthand, so markup reads `{t('nav.overview')}`. */
export function t(key: MessageKey, params?: Record<string, string | number>): string {
	return i18n.t(key, params);
}
