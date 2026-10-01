<script lang="ts">
	/**
	 * A chart the reader can re-shape: what form it takes, and which series it
	 * draws. Same data, three readings — a stack answers "how much altogether",
	 * bars side by side answer "which is bigger", a line answers "where it is
	 * going".
	 *
	 * Drawn by layerchart through the shadcn wrapper, so the tooltip, the
	 * clickable legend and the axis ticks are the library's rather than this
	 * app's. Callers still pass labels and series — every page here holds its
	 * numbers that way, and layerchart wants one row per label.
	 */
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChartArea from '@lucide/svelte/icons/chart-area';
	import ChartColumn from '@lucide/svelte/icons/chart-column';
	import ChartColumnStacked from '@lucide/svelte/icons/chart-column-stacked';
	import ChartLine from '@lucide/svelte/icons/chart-line';
	import { scaleBand, scalePoint } from 'd3-scale';
	import { AreaChart, BarChart } from 'layerchart';
	import { Button } from '$lib/components/ui/button';
	import * as Chart from '$lib/components/ui/chart';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { shortLabel } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import type { MessageKey } from '$lib/i18n/en';

	export type ChartType = 'stacked' | 'bars' | 'line' | 'area';

	type Entry = { key: string; values: (number | null)[]; dashed?: boolean };

	const CHART_VARS = ['--chart-1', '--chart-2', '--chart-3', '--chart-4', '--chart-5'];

	type Props = {
		labels: string[];
		series: Entry[];
		format: (value: number) => string;
		/** The shape the data reads best in before anyone touches the control. */
		type?: ChartType;
		height?: number;
		/** Shortens a key for the legend; the data keeps the full name. */
		display?: (key: string) => string;
		/** The shapes worth offering. A chart whose series must not be added
		 *  together has no business offering a stack. */
		types?: ChartType[];
	};

	let {
		labels,
		series,
		format,
		type = 'stacked',
		height = 200,
		display = (key) => key,
		types
	}: Props = $props();

	const ALL: { id: ChartType; label: MessageKey; icon: typeof ChartColumn }[] = [
		{ id: 'stacked', label: 'chart.type.stacked', icon: ChartColumnStacked },
		{ id: 'bars', label: 'chart.type.bars', icon: ChartColumn },
		{ id: 'line', label: 'chart.type.line', icon: ChartLine },
		{ id: 'area', label: 'chart.type.area', icon: ChartArea }
	];

	const TYPES = $derived(types ? ALL.filter((entry) => types.includes(entry.id)) : ALL);

	/** Null until the reader picks, so the caller's default still applies. */
	let picked = $state<ChartType | null>(null);

	const chosen = $derived(picked ?? type);
	const current = $derived(TYPES.find((entry) => entry.id === chosen) ?? TYPES[0]);

	const colour = (index: number) => `var(${CHART_VARS[index % CHART_VARS.length]})`;

	// Room for the labels rather than none: a chart that draws to its own edge
	// puts the axis text outside the card it sits in. The bottom holds two
	// rows, not one — layerchart pins the legend to the floor of the chart, and
	// with only the axis allowed for it landed across the dates.
	const PADDING = { top: 8, right: 24, bottom: 44, left: 56 };

	/**
	 * How the x axis is drawn: shortened labels, and only as many of them as
	 * fit side by side.
	 *
	 * layerchart shows every value of a band scale unless it is told a spacing,
	 * so a year by day arrived as three hundred labels printed over each other.
	 * The tooltip still carries the full label, which is where an exact date
	 * belongs.
	 */
	const X_AXIS = { format: shortLabel, tickSpacing: 64 };

	/** One row per label, which is the shape layerchart reads. */
	const data = $derived(
		labels.map((label, index) => {
			const row: Record<string, string | number | null> = { label };
			for (const entry of series) row[entry.key] = entry.values[index] ?? null;
			return row;
		})
	);

	const drawn = $derived(
		series.map((entry, index) => ({
			key: entry.key,
			label: display(entry.key),
			color: colour(index)
		}))
	);

	/** What the tooltip and the legend call each series, and in which colour. */
	const config = $derived(
		Object.fromEntries(
			series.map((entry, index) => [entry.key, { label: display(entry.key), color: colour(index) }])
		) as Chart.ChartConfig
	);
</script>

<div class="flex flex-col gap-2">
	<div class="flex flex-wrap items-center gap-2">
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button {...props} variant="outline" size="sm" class="h-8 gap-1 font-normal">
						<current.icon class="size-3.5" />
						{t(current.label)}
						<ChevronDown class="size-3.5 opacity-60" />
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content align="start" class="w-48">
				{#each TYPES as entry (entry.id)}
					<DropdownMenu.Item onSelect={() => (picked = entry.id)}>
						<entry.icon class="size-4 text-muted-foreground" />
						<span class="flex-1">{t(entry.label)}</span>
						{#if chosen === entry.id}
							<Check class="size-4" />
						{/if}
					</DropdownMenu.Item>
				{/each}
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</div>

	{#if series.length === 0}
		<p class="py-8 text-center text-xs text-muted-foreground">{t('chart.noSeries')}</p>
	{:else}
		<!-- The legend is the library's, and it is the control: a click on a name
		     puts that series away and back. -->
		<Chart.Container {config} class="aspect-auto w-full" style="height: {height}px">
			{#if chosen === 'line' || chosen === 'area'}
				<AreaChart
					legend
					padding={PADDING}
					{data}
					x="label"
					xScale={scalePoint()}
					series={drawn}
					seriesLayout="overlap"
					props={{
						area: { line: { class: 'stroke-2' }, fillOpacity: chosen === 'area' ? 0.25 : 0 },
						xAxis: X_AXIS,
						yAxis: { format }
					}}
				>
					{#snippet tooltip()}
						<Chart.Tooltip indicator="line" />
					{/snippet}
				</AreaChart>
			{:else}
				<BarChart
					legend
					padding={PADDING}
					{data}
					x="label"
					xScale={scaleBand().padding(0.25)}
					series={drawn}
					seriesLayout={chosen === 'stacked' ? 'stack' : 'group'}
					props={{ xAxis: X_AXIS, yAxis: { format } }}
				>
					{#snippet tooltip()}
						<Chart.Tooltip />
					{/snippet}
				</BarChart>
			{/if}
		</Chart.Container>
	{/if}
</div>
