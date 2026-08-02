/**
 * Search, per-column filters and multi-column sort for the tables the host
 * hands over whole.
 *
 * Sessions is paged and sorted by the host, because a long history does not fit
 * in the window. Tools, models, agents and projects arrive as one array of a few
 * dozen rows — sorting those in SQL would mean a round trip per click for no
 * gain, so they are sorted here.
 */

/** Sortable values. `null` always sinks to the bottom, whichever way it points. */
export type Cell = string | number | null;

export type TableColumns<T> = Record<string, (row: T) => Cell>;

export type SortEntry = { id: string; descending: boolean };

/** How a column offers to be filtered, decided from the values it holds. */
export type FilterKind = 'range' | 'list' | 'text';

/** Above this many distinct values a checkbox list stops being a list. */
const LIST_LIMIT = 40;

function compare(a: Cell, b: Cell): number {
	if (a === null && b === null) return 0;
	if (a === null) return 1;
	if (b === null) return -1;
	if (typeof a === 'number' && typeof b === 'number') return a - b;
	return String(a).localeCompare(String(b));
}

function matches(value: Cell, needle: string): boolean {
	return String(value ?? '')
		.toLowerCase()
		.includes(needle);
}

export function createTable<T>(
	source: () => T[],
	columns: TableColumns<T>,
	initial: { sort: string; descending?: boolean }
) {
	let query = $state('');
	/** Chosen values per column, for the list kind. */
	let selected = $state<Record<string, string[]>>({});
	/** Free text per column, for the text kind. */
	let text = $state<Record<string, string>>({});
	/** Inclusive bounds per column, for the range kind. Empty means open. */
	let ranges = $state<Record<string, { min: string; max: string }>>({});
	let sorts = $state<SortEntry[]>([{ id: initial.sort, descending: initial.descending ?? true }]);

	function distinct(id: string, rows: T[]): string[] {
		const read = columns[id];
		if (!read) return [];
		const seen: string[] = [];
		for (const row of rows) {
			const value = String(read(row) ?? '');
			if (!seen.includes(value)) seen.push(value);
		}
		return seen.sort((a, b) => a.localeCompare(b));
	}

	/** Decided from the data, not declared twice: a column that holds numbers
	 *  wants a range, one with few repeated values wants a list. */
	function kind(id: string): FilterKind {
		const read = columns[id];
		if (!read) return 'text';
		const rows = source();
		const sample = rows.map(read).find((value) => value !== null);
		if (typeof sample === 'number') return 'range';
		return distinct(id, rows).length <= LIST_LIMIT ? 'list' : 'text';
	}

	const filtered = $derived.by(() => {
		const needle = query.trim().toLowerCase();

		return source().filter((row) => {
			// Column filters narrow together; within one column the chosen values
			// widen, which is what a checkbox list leads people to expect.
			for (const [id, values] of Object.entries(selected)) {
				if (values.length === 0) continue;
				const read = columns[id];
				if (read && !values.includes(String(read(row) ?? ''))) return false;
			}

			for (const [id, value] of Object.entries(text)) {
				if (value.trim() === '') continue;
				const read = columns[id];
				if (read && !matches(read(row), value.trim().toLowerCase())) return false;
			}

			for (const [id, bounds] of Object.entries(ranges)) {
				const read = columns[id];
				if (!read) continue;
				const value = read(row);
				if (typeof value !== 'number') {
					// A row with no number cannot satisfy a bound, so a set bound
					// excludes it rather than letting it drift to the top.
					if (bounds.min !== '' || bounds.max !== '') return false;
					continue;
				}
				if (bounds.min !== '' && value < Number(bounds.min)) return false;
				if (bounds.max !== '' && value > Number(bounds.max)) return false;
			}

			if (!needle) return true;
			return Object.values(columns).some((read) => matches(read(row), needle));
		});
	});

	const rows = $derived.by(() => {
		if (sorts.length === 0) return filtered;
		// A copy: sorting the array from the store in place would mutate what the
		// caller still holds.
		return [...filtered].sort((a, b) => {
			for (const entry of sorts) {
				const read = columns[entry.id];
				if (!read) continue;
				const result = compare(read(a), read(b));
				if (result !== 0) return entry.descending ? -result : result;
			}
			return 0;
		});
	});

	return {
		get query() {
			return query;
		},
		set query(value: string) {
			query = value;
		},
		get sorts() {
			return sorts;
		},
		get rows() {
			return rows;
		},
		kind,
		/**
		 * The values a column offers, narrowed by every *other* column's filter.
		 *
		 * Excluding the column's own selection is what lets a chosen value be
		 * unchecked again — filtering the list by itself would leave one option.
		 */
		options(id: string): string[] {
			const others = Object.entries(selected).filter(
				([key, values]) => key !== id && values.length > 0
			);
			const rows = source().filter((row) =>
				others.every(([key, chosen]) => {
					const other = columns[key];
					return !other || chosen.includes(String(other(row) ?? ''));
				})
			);
			return distinct(id, rows);
		},
		chosen(id: string): string[] {
			return selected[id] ?? [];
		},
		toggleValue(id: string, value: string) {
			const current = selected[id] ?? [];
			selected = {
				...selected,
				[id]: current.includes(value)
					? current.filter((entry) => entry !== value)
					: [...current, value]
			};
		},
		textFilter(id: string): string {
			return text[id] ?? '';
		},
		setText(id: string, value: string) {
			text = { ...text, [id]: value };
		},
		range(id: string): { min: string; max: string } {
			return ranges[id] ?? { min: '', max: '' };
		},
		setRange(id: string, bound: 'min' | 'max', value: string) {
			const current = ranges[id] ?? { min: '', max: '' };
			ranges = { ...ranges, [id]: { ...current, [bound]: value } };
		},
		/** True while the column narrows anything, whichever kind it is. */
		isFiltered(id: string): boolean {
			const bounds = ranges[id];
			return (
				(selected[id]?.length ?? 0) > 0 ||
				(text[id]?.trim() ?? '') !== '' ||
				(bounds !== undefined && (bounds.min !== '' || bounds.max !== ''))
			);
		},
		clearFilter(id: string) {
			selected = { ...selected, [id]: [] };
			text = { ...text, [id]: '' };
			ranges = { ...ranges, [id]: { min: '', max: '' } };
		},
		/** Where a column sits in the sort order, or 0 when it does not sort. */
		rank(id: string): number {
			return sorts.findIndex((entry) => entry.id === id) + 1;
		},
		direction(id: string): 'asc' | 'desc' | null {
			const entry = sorts.find((item) => item.id === id);
			return entry ? (entry.descending ? 'desc' : 'asc') : null;
		},
		/**
		 * Cycles one column: descending, ascending, then out of the sort.
		 *
		 * Columns accumulate in the order they were clicked, so a second column
		 * breaks the first one's ties rather than replacing it. Dropping out on
		 * the third click is what keeps that from becoming a one-way ratchet.
		 */
		toggle(id: string) {
			const index = sorts.findIndex((entry) => entry.id === id);
			if (index === -1) {
				sorts = [...sorts, { id, descending: true }];
			} else if (sorts[index].descending) {
				sorts = sorts.map((entry, i) => (i === index ? { id, descending: false } : entry));
			} else {
				sorts = sorts.filter((entry) => entry.id !== id);
			}
		}
	};
}
