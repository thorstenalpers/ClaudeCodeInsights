const COMPACT = new Intl.NumberFormat('en-US', {
	notation: 'compact',
	maximumFractionDigits: 1
});

const EXACT = new Intl.NumberFormat('en-US');

/** Short form for headline figures: 1.1B, 4.2K. */
export function compact(value: number): string {
	return COMPACT.format(value);
}

/** Full form, for tooltips and anywhere the exact number matters. */
export function exact(value: number): string {
	return EXACT.format(value);
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
	return `${value.toFixed(value >= 100 ? 0 : 1)} ${units[unit]}`;
}

export function formatDate(iso: string | null): string {
	if (!iso) return '—';
	const date = new Date(iso);
	return Number.isNaN(date.getTime()) ? '—' : date.toLocaleDateString();
}
