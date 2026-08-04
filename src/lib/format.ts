import { i18n } from '$lib/i18n/index.svelte';

// Formatters are rebuilt when the language changes rather than cached once:
// a number in the wrong thousands separator is as wrong as an untranslated
// label, and switching languages is rare enough that the cost is invisible.
function compactFormat(): Intl.NumberFormat {
	return new Intl.NumberFormat(i18n.intlLocale, {
		notation: 'compact',
		maximumFractionDigits: 1
	});
}

/** Short form for headline figures: 1.1B, 4.2K. */
export function compact(value: number): string {
	return compactFormat().format(value);
}

/** Full form, for tooltips and anywhere the exact number matters. */
export function exact(value: number): string {
	return new Intl.NumberFormat(i18n.intlLocale).format(value);
}

export function formatBytes(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	const units = ['KB', 'MB', 'GB'];
	let value = bytes;
	let unit = -1;
	do {
		value /= 1024;
		unit += 1;
	} while (value >= 1024 && unit < units.length - 1);
	return `${new Intl.NumberFormat(i18n.intlLocale, {
		maximumFractionDigits: value >= 100 ? 0 : 1
	}).format(value)} ${units[unit]}`;
}

export function formatDate(iso: string | null): string {
	if (!iso) return '—';
	const date = new Date(iso);
	return Number.isNaN(date.getTime()) ? '—' : date.toLocaleDateString(i18n.intlLocale);
}

/** Day, month and time — the resolution the tables need. */
export function formatWhen(iso: string | null): string {
	if (!iso) return '—';
	const date = new Date(iso);
	if (Number.isNaN(date.getTime())) return '—';
	return date.toLocaleString(i18n.intlLocale, {
		month: 'short',
		day: 'numeric',
		hour: '2-digit',
		minute: '2-digit'
	});
}

export function formatTime(iso: string | null): string {
	if (!iso) return '';
	const date = new Date(iso);
	return Number.isNaN(date.getTime())
		? ''
		: date.toLocaleTimeString(i18n.intlLocale, { hour: '2-digit', minute: '2-digit' });
}

/**
 * The clock down to the second.
 *
 * A live stream puts several lines inside the same minute, and without the
 * seconds they read as one moment: the order is visible but not the pace.
 */
export function formatSecond(iso: string | null): string {
	if (!iso) return '';
	const date = new Date(iso);
	return Number.isNaN(date.getTime())
		? ''
		: date.toLocaleTimeString(i18n.intlLocale, {
				hour: '2-digit',
				minute: '2-digit',
				second: '2-digit'
			});
}

/**
 * One spelling for every Windows path on screen.
 *
 * Both separators reach the same directory, and a list that mixes them reads
 * like two different places. Only paths that name a drive are rewritten: a
 * POSIX path is a path on another system, and turning its slashes round would
 * make it wrong rather than tidy.
 */
export function displayPath(path: string): string {
	return /^[a-zA-Z]:/.test(path) ? path.replace(/\//g, '\\') : path;
}
