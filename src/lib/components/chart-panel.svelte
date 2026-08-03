<script lang="ts">
	/**
	 * A chart the reader can re-shape: what form it takes, and which series it
	 * draws. Same data, three readings — a stack answers "how much altogether",
	 * bars side by side answer "which is bigger", a line answers "where it is
	 * going".
	 */
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChartArea from '@lucide/svelte/icons/chart-area';
	import ChartColumn from '@lucide/svelte/icons/chart-column';
	import ChartColumnStacked from '@lucide/svelte/icons/chart-column-stacked';
	import ChartLine from '@lucide/svelte/icons/chart-line';
	import BarChart from '$lib/components/bar-chart.svelte';
	import LineChart from '$lib/components/line-chart.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { t } from '$lib/i18n/index.svelte';
	import type { MessageKey } from '$lib/i18n/en';

	export type ChartType = 'stacked' | 'bars' | 'line' | 'area';

	type Entry = { key: string; values: number[]; dashed?: boolean };

	type Props = {
		labels: string[];
		series: Entry[];
		format: (value: number) => string;
		/** The shape the data reads best in before anyone touches the control. */
		type?: ChartType;
		height?: number;
		/** Shortens a key for the menu; the chart itself keeps the full name. */
		display?: (key: string) => string;
	};

	let {
		labels,
		series,
		format,
		type = 'stacked',
		height,
		display = (key) => key
	}: Props = $props();

	const TYPES: { id: ChartType; label: MessageKey; icon: typeof ChartColumn }[] = [
		{ id: 'stacked', label: 'chart.type.stacked', icon: ChartColumnStacked },
		{ id: 'bars', label: 'chart.type.bars', icon: ChartColumn },
		{ id: 'line', label: 'chart.type.line', icon: ChartLine },
		{ id: 'area', label: 'chart.type.area', icon: ChartArea }
	];

	/** Null until the reader picks, so the caller's default still applies. */
	let picked = $state<ChartType | null>(null);

	// Hidden rather than shown: a series that appears later — a model used for
	// the first time — is drawn instead of silently missing.
	let hidden = $state<string[]>([]);

	const chosen = $derived(picked ?? type);
	const current = $derived(TYPES.find((entry) => entry.id === chosen) ?? TYPES[0]);
	const drawn = $derived(series.filter((entry) => !hidden.includes(entry.key)));

	function toggle(key: string) {
		hidden = hidden.includes(key) ? hidden.filter((entry) => entry !== key) : [...hidden, key];
	}
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

		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant={hidden.length > 0 ? 'default' : 'outline'}
						size="sm"
						class="h-8 gap-1 font-normal"
						disabled={series.length === 0}
					>
						{t('chart.series')}
						<span class="tabular-nums">{drawn.length}/{series.length}</span>
						<ChevronDown class="size-3.5 opacity-60" />
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content align="start" class="max-h-80 w-56 overflow-y-auto">
				<DropdownMenu.Label class="flex items-center justify-between gap-2">
					{t('chart.series')}
					{#if hidden.length > 0}
						<button
							type="button"
							class="text-xs font-normal text-muted-foreground hover:text-foreground"
							onclick={() => (hidden = [])}
						>
							{t('chart.series.all')}
						</button>
					{/if}
				</DropdownMenu.Label>
				<DropdownMenu.Separator />

				{#each series as entry (entry.key)}
					<DropdownMenu.CheckboxItem
						checked={!hidden.includes(entry.key)}
						onCheckedChange={() => toggle(entry.key)}
						closeOnSelect={false}
					>
						<span class="truncate">{display(entry.key)}</span>
					</DropdownMenu.CheckboxItem>
				{/each}
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</div>

	{#if drawn.length === 0}
		<p class="py-8 text-center text-xs text-muted-foreground">{t('chart.noSeries')}</p>
	{:else if chosen === 'line' || chosen === 'area'}
		<LineChart {labels} series={drawn} {format} {height} area={chosen === 'area'} />
	{:else}
		<BarChart {labels} series={drawn} {format} stacked={chosen === 'stacked'} {height} />
	{/if}
</div>
