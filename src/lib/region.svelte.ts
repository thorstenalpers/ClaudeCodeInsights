/**
 * The country whose currency the figures are shown in.
 *
 * The rates are published in USD, and this app never reaches the network — so
 * anything other than dollars needs an exchange rate the user supplies. That is
 * a deliberate trade: a stale rate fetched once and cached silently would be
 * worse than a number the user knows they set.
 */
import { i18n, type Locale } from '$lib/i18n/index.svelte';

/** The currency is the choice; a country would only be a longer way to say it. */
export const CURRENCIES = [
	'USD',
	'EUR',
	'GBP',
	'CHF',
	'CAD',
	'AUD',
	'BRL',
	'INR',
	'CNY',
	'JPY',
	'RUB',
	'AED'
] as const;

export type Currency = (typeof CURRENCIES)[number];
export type CurrencySetting = Currency | 'system';

/** Where a language points when the user has not said otherwise. */
const DEFAULT_CURRENCY: Record<Locale, Currency> = {
	en: 'USD',
	de: 'EUR',
	fr: 'EUR',
	es: 'EUR',
	it: 'EUR',
	ja: 'JPY',
	pt: 'EUR',
	zh: 'CNY',
	ru: 'RUB',
	hi: 'INR',
	ar: 'AED'
};

const CURRENCY_KEY = 'claudeadmin.currency';
const RATE_KEY = 'claudeadmin.rate';

function isCurrency(value: unknown): value is Currency {
	return CURRENCIES.some((entry) => entry === value);
}

function readStoredCurrency(): CurrencySetting {
	if (typeof localStorage === 'undefined') return 'system';
	const stored = localStorage.getItem(CURRENCY_KEY);
	return stored === 'system' || isCurrency(stored) ? stored : 'system';
}

function readStoredRates(): Record<string, number> {
	if (typeof localStorage === 'undefined') return {};
	try {
		const raw: unknown = JSON.parse(localStorage.getItem(RATE_KEY) ?? '{}');
		if (typeof raw !== 'object' || raw === null) return {};
		return Object.fromEntries(
			Object.entries(raw as Record<string, unknown>).filter(
				([, value]) => typeof value === 'number' && value > 0
			) as [string, number][]
		);
	} catch {
		return {};
	}
}

class Region {
	setting = $state<CurrencySetting>(readStoredCurrency());
	/** Target units per US dollar, per currency. */
	rates = $state<Record<string, number>>(readStoredRates());

	get currency(): Currency {
		return this.setting === 'system' ? DEFAULT_CURRENCY[i18n.locale] : this.setting;
	}

	/** Dollars are the source, so they never need converting. */
	get rate(): number {
		return this.currency === 'USD' ? 1 : (this.rates[this.currency] ?? 1);
	}

	/** True while a non-dollar currency is showing dollar amounts unconverted. */
	get needsRate(): boolean {
		return this.currency !== 'USD' && this.rates[this.currency] === undefined;
	}

	set(next: CurrencySetting): void {
		this.setting = next;
		if (next === 'system') {
			localStorage.removeItem(CURRENCY_KEY);
		} else {
			localStorage.setItem(CURRENCY_KEY, next);
		}
	}

	setRate(value: number | null): void {
		const next = { ...this.rates };
		if (value === null || !Number.isFinite(value) || value <= 0) {
			delete next[this.currency];
		} else {
			next[this.currency] = value;
		}
		this.rates = next;
		localStorage.setItem(RATE_KEY, JSON.stringify(next));
	}

	/** Formats a USD amount in the chosen currency, converting if a rate is set. */
	format(usd: number): string {
		const value = usd * this.rate;
		return new Intl.NumberFormat(i18n.intlLocale, {
			style: 'currency',
			currency: this.currency,
			maximumFractionDigits: value < 10 ? 2 : 0
		}).format(value);
	}
}

export const region = new Region();
