/**
 * The active language, and the lookup every view uses.
 *
 * There is no i18n library behind this: flat catalogs and a lookup are all the
 * app needs, and the English catalog doubles as the type, so a key that exists
 * nowhere is a compile error rather than a blank spot in the window.
 */
import { ar } from './ar';
import { de } from './de';
import { en, type MessageKey, type Messages } from './en';
import { es } from './es';
import { fr } from './fr';
import { hi } from './hi';
import { it } from './it';
import { ja } from './ja';
import { pt } from './pt';
import { ru } from './ru';
import { zh } from './zh';

/** Listed in the order the menu shows them: English first, then by endonym. */
export const LOCALES = [
	{ id: 'en', label: 'English' },
	{ id: 'ar', label: 'العربية' },
	{ id: 'de', label: 'Deutsch' },
	{ id: 'es', label: 'Español' },
	{ id: 'fr', label: 'Français' },
	{ id: 'hi', label: 'हिन्दी' },
	{ id: 'it', label: 'Italiano' },
	{ id: 'pt', label: 'Português' },
	{ id: 'ja', label: '日本語' },
	{ id: 'ru', label: 'Русский' },
	{ id: 'zh', label: '中文' }
] as const;

export type Locale = (typeof LOCALES)[number]['id'];
/** 'system' follows the OS; anything else is the user's explicit choice. */
export type LocaleSetting = Locale | 'system';

const CATALOGS: Record<Locale, Messages> = { en, ar, de, es, fr, hi, it, ja, pt, ru, zh };

/** Scripts that run right to left. The layout mirrors for these. */
const RTL: ReadonlySet<string> = new Set(['ar']);

const INTL_TAGS: Record<Locale, string> = {
	en: 'en-US',
	ar: 'ar-EG',
	de: 'de-DE',
	es: 'es-ES',
	fr: 'fr-FR',
	hi: 'hi-IN',
	it: 'it-IT',
	ja: 'ja-JP',
	pt: 'pt-PT',
	ru: 'ru-RU',
	zh: 'zh-CN'
};

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
		return INTL_TAGS[this.locale];
	}

	get isRtl(): boolean {
		return RTL.has(this.locale);
	}

	set(next: LocaleSetting): void {
		this.setting = next;
		if (next === 'system') {
			localStorage.removeItem(STORAGE_KEY);
		} else {
			localStorage.setItem(STORAGE_KEY, next);
		}
		this.applyDocumentLanguage();
	}

	/** Looks a message up, filling `{placeholders}` from `params`. */
	t(key: MessageKey, params?: Record<string, string | number>): string {
		const message = CATALOGS[this.locale][key] ?? en[key];
		if (!params) return message;
		return message.replace(/\{(\w+)\}/g, (whole, name: string) =>
			name in params ? String(params[name]) : whole
		);
	}

	/** `lang` for screen readers and hyphenation, `dir` so Arabic mirrors. */
	private applyDocumentLanguage(): void {
		const root = document.documentElement;
		root.lang = this.locale;
		root.dir = this.isRtl ? 'rtl' : 'ltr';
	}

	init(): void {
		this.applyDocumentLanguage();
	}
}

export const i18n = new I18n();

/** Shorthand, so markup reads `{t('nav.overview')}`. */
export function t(key: MessageKey, params?: Record<string, string | number>): string {
	return i18n.t(key, params);
}
