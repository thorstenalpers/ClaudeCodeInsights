<script lang="ts">
	import * as Tooltip from '$lib/components/ui/tooltip';

	type Series = { key: string; values: number[] };

	type Props = {
		/** One label per column, in order. */
		labels: string[];
		/** Stacked bottom to top, in the order given. */
		series: Series[];
		/** Turns a value into what the tooltip shows. */
		format: (value: number) => string;
		height?: number;
		/** False puts the series side by side, which is what comparing them needs. */
		stacked?: boolean;
	};

	let { labels, series, format, height = 180, stacked = true }: Props = $props();

	const CHART_VARS = ['--chart-1', '--chart-2', '--chart-3', '--chart-4', '--chart-5'];

	const totals = $derived(
		labels.map((_, index) => series.reduce((sum, entry) => sum + (entry.values[index] ?? 0), 0))
	);

	// A chart of nothing should be flat rather than infinite, so an all-zero
	// series still gets a scale. Side by side, a column is only as tall as its
	// tallest bar, so the stack's total would leave every bar half drawn.
	const peak = $derived(
		Math.max(1, ...(stacked ? totals : series.flatMap((entry) => entry.values)))
	);
</script>

<div class="flex w-full items-end gap-1" style="height: {height}px">
	{#each labels as label, index (label)}
		{@const total = totals[index]}
		<Tooltip.Provider>
			<Tooltip.Root>
				<Tooltip.Trigger class="flex h-full flex-1 flex-col justify-end">
					{#snippet child({ props })}
						<div {...props} class="flex h-full min-w-1 flex-1 flex-col justify-end">
							{#if stacked}
								{#each series as entry, layer (entry.key)}
									{@const value = entry.values[index] ?? 0}
									{#if value > 0}
										<div
											class="w-full first:rounded-t-sm"
											style="height: {(value / peak) * 100}%; background: var({CHART_VARS[
												layer % CHART_VARS.length
											]})"
										></div>
									{/if}
								{/each}
							{:else}
								<div class="flex h-full w-full items-end gap-px">
									{#each series as entry, layer (entry.key)}
										{@const value = entry.values[index] ?? 0}
										<div
											class="min-w-px flex-1 rounded-t-sm"
											style="height: {(value / peak) * 100}%; background: var({CHART_VARS[
												layer % CHART_VARS.length
											]})"
										></div>
									{/each}
								</div>
							{/if}
							{#if total === 0}
								<div class="h-px w-full bg-border"></div>
							{/if}
						</div>
					{/snippet}
				</Tooltip.Trigger>

				<Tooltip.Content>
					<div class="flex flex-col gap-0.5 text-xs">
						<span class="font-medium">{label}</span>
						{#each series as entry, layer (entry.key)}
							{@const value = entry.values[index] ?? 0}
							{#if value > 0}
								<div class="flex items-center gap-2">
									<span
										class="size-2 shrink-0 rounded-[2px]"
										style="background: var({CHART_VARS[layer % CHART_VARS.length]})"
									></span>
									<span class="flex-1">{entry.key}</span>
									<span class="tabular-nums">{format(value)}</span>
								</div>
							{/if}
						{/each}
						<span class="mt-0.5 border-t pt-0.5 text-right tabular-nums">{format(total)}</span>
					</div>
				</Tooltip.Content>
			</Tooltip.Root>
		</Tooltip.Provider>
	{/each}
</div>

<div class="mt-1 flex w-full gap-1 text-[10px] text-muted-foreground">
	{#each labels as label, index (label)}
		<span class="flex-1 truncate text-center">
			{index === 0 || index === labels.length - 1 || labels.length <= 12 ? label : ''}
		</span>
	{/each}
</div>
