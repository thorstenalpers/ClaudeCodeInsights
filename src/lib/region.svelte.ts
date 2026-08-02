/**
 * The country whose currency the figures are shown in.
 *
 * The rates are published in USD, and this app never reaches the network — so
 * anything other than dollars needs an exchange rate the user supplies. That is
 * a deliberate trade: a stale rate fetched once and cached silently would be
 * worse than a number the user knows they set.
 */
import { i18n, type Locale } from '$lib/i18n/index.svelte';

export const REGIONS = [
	{ id: 'US', currency: 'USD' },
	{ id: 'GB', currency: 'GBP' },
	{ id: 'DE', currency: 'EUR' },
	{ id: 'AT', currency: 'EUR' },
	{ id: 'CH', currency: 'CHF' },
	{ id: 'FR', currency: 'EUR' },
	{ id: 'ES', currency: 'EUR' },
	{ id: 'IT', currency: 'EUR' },
	{ id: 'CA', currency: 'CAD' },
	{ id: 'AU', currency: 'AUD' },
	{ id: 'BR', currency: 'BRL' },
	{ id: 'PT', currency: 'EUR' },
	{ id: 'IN', currency: 'INR' },
	{ id: 'CN', currency: 'CNY' },
	{ id: 'JP', currency: 'JPY' },
	{ id: 'RU', currency: 'RUB' },
	{ id: 'AE', currency: 'AED' }
] as const;

export type RegionId = (typeof REGIONS)[number]['id'];
export type RegionSetting = RegionId | 'system';

/** Where a language points when the user has not said otherwise. */
const DEFAULT_REGION: Record<Locale, RegionId> = {
	en: 'US',
	de: 'DE',
	fr: 'FR',
	es: 'ES',
	it: 'IT',
	ja: 'JP',
	pt: 'PT',
	zh: 'CN',
	ru: 'RU',
	hi: 'IN',
	ar: 'AE'
};

const REGION_KEY = 'claudeadmin.region';
const RATE_KEY = 'claudeadmin.rate';

function isRegion(value: unknown): value is RegionId {
	return REGIONS.some((region) => region.id === value);
}

function readStoredRegion(): RegionSetting {
	if (typeof localStorage === 'undefined') return 'system';
	const stored = localStorage.getItem(REGION_KEY);
	return stored === 'system' || isRegion(stored) ? stored : 'system';
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
	setting = $state<RegionSetting>(readStoredRegion());
	/** Target units per US dollar, per currency. */
	rates = $state<Record<string, number>>(readStoredRates());

	get id(): RegionId {
		return this.setting === 'system' ? DEFAULT_REGION[i18n.locale] : this.setting;
	}

	get currency(): string {
		return REGIONS.find((region) => region.id === this.id)?.currency ?? 'USD';
	}

	/** Dollars are the source, so they never need converting. */
	get rate(): number {
		return this.currency === 'USD' ? 1 : (this.rates[this.currency] ?? 1);
	}

	/** True while a non-dollar currency is showing dollar amounts unconverted. */
	get needsRate(): boolean {
		return this.currency !== 'USD' && this.rates[this.currency] === undefined;
	}

	set(next: RegionSetting): void {
		this.setting = next;
		if (next === 'system') {
			localStorage.removeItem(REGION_KEY);
		} else {
			localStorage.setItem(REGION_KEY, next);
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
