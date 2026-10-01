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

/** Wall-clock length of a session, not time spent working. */
export function formatDuration(minutes: number): string {
	if (minutes < 1) return '<1m';
	if (minutes < 60) return `${minutes}m`;
	return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
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
 * A bucket label short enough to sit on an axis.
 *
 * `2025-06` and `2025-06-15` are wider than the column they belong to, and a
 * row of them ends up on top of each other; two numbers still say which month
 * or which day it is. Anything that is not a date is left alone — the same axis
 * carries model names and tool names.
 *
 * Read as UTC, because these labels are calendar buckets rather than moments:
 * a local reading of `2025-06-01` is May somewhere.
 */
export function shortLabel(label: string): string {
	const parts = /^(\d{4})-(\d{2})(?:-(\d{2}))?$/.exec(label);
	if (!parts) return label;

	const [, year, month, day] = parts;
	const date = new Date(Date.UTC(Number(year), Number(month) - 1, Number(day ?? '1')));
	return date.toLocaleDateString(i18n.intlLocale, {
		timeZone: 'UTC',
		month: '2-digit',
		...(day ? { day: '2-digit' } : { year: '2-digit' })
	});
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

/**
 * The product name behind a source id. A product name, not a translation —
 * it is the same in every language, which is why it does not live in i18n.
 */
export function sourceName(source: string): string {
	return { claude: 'Claude Code', codex: 'Codex' }[source] ?? source;
}
