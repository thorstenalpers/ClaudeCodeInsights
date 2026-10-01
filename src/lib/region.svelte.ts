/**
 * Every figure in US dollars.
 *
 * The rates are published in USD, and this app never reaches the network — a
 * conversion would need an exchange rate somebody has to keep true. Showing
 * the source currency everywhere costs a familiarity, and buys figures that
 * are never silently wrong.
 */
import { i18n } from '$lib/i18n/index.svelte';

class Region {
	readonly currency = 'USD';
	/** Dollars are the source, so nothing ever needs converting. */
	readonly rate = 1;
	readonly isIndicative = false;

	/** Formats a USD amount in the window's language, always as dollars. */
	format(usd: number): string {
		return new Intl.NumberFormat(i18n.intlLocale, {
			style: 'currency',
			currency: 'USD',
			maximumFractionDigits: usd < 10 ? 2 : 0
		}).format(usd);
	}
}

export const region = new Region();
