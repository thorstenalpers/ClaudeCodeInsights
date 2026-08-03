<script lang="ts">
	/** This project alone, over time — the same series the cost page draws for
	 *  everything, filtered to one path by the host. */
	import { api, type Series } from '$lib/api';
	import ChartPanel from '$lib/components/chart-panel.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { costOf } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';

	type Props = { path: string };
	let { path }: Props = $props();

	type Grain = 'month' | 'day';
	const GRAINS: Grain[] = ['month', 'day'];

	let grain = $state<Grain>('month');
	let series = $state<Series | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	$effect(() => {
		const wanted = path;
		void scan.dataVersion;
		if (!isHosted) {
			loading = false;
			return;
		}

		let cancelled = false;
		error = null;
		api
			.getSeries({ groupBy: 'model', projects: [wanted] })
			.then((value) => {
				if (!cancelled) series = value;
			})
			.catch((cause) => {
				if (!cancelled) error = errorMessage(cause);
			})
			.finally(() => {
				if (!cancelled) loading = false;
			});

		return () => {
			cancelled = true;
		};
	});

	const history = $derived.by(() => {
		if (!series)
			return { labels: [] as string[], series: [] as { key: string; values: number[] }[] };

		const column = (date: string) => (grain === 'month' ? date.slice(0, 7) : date);

		const labels: string[] = [];
		for (const point of series.points) {
			const label = column(point.date);
			if (!labels.includes(label)) labels.push(label);
		}
		labels.sort();

		const byKey: Record<string, number[]> = {};
		for (const point of series.points) {
			const index = labels.indexOf(column(point.date));
			if (index === -1) continue;
			byKey[point.key] ??= Array<number>(labels.length).fill(0);
			byKey[point.key][index] += costOf(point.model, point);
		}

		return {
			labels,
			series: Object.entries(byKey)
				.map(([key, values]) => ({ key: key.replace(/^claude-/, ''), values }))
				.sort((a, b) => b.values.reduce((x, y) => x + y, 0) - a.values.reduce((x, y) => x + y, 0))
		};
	});

	function money(value: number): string {
		return region.format(value);
	}
</script>

<div class="@container flex flex-col gap-3 p-4">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('overview.dbFailed')}</Card.Title>
				<Card.Description class="font-mono text-xs">{error}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if loading}
		<Skeleton class="h-64 w-full" />
	{:else if history.labels.length === 0}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('cost.empty')}</Card.Title>
				<Card.Description>{t('sessions.emptyScan')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else}
		<Card.Root data-size="sm">
			<Card.Header class="gap-1">
				<div class="flex flex-wrap items-center justify-between gap-2">
					<Card.Title class="text-base">{t('cost.history')}</Card.Title>
					<div class="flex gap-1">
						{#each GRAINS as option (option)}
							<Button
								variant={grain === option ? 'default' : 'outline'}
								size="sm"
								class="h-8 font-normal"
								onclick={() => (grain = option)}
							>
								{t(option === 'month' ? 'cost.byMonth' : 'cost.byDay')}
							</Button>
						{/each}
					</div>
				</div>
				<Card.Description>
					{t(grain === 'month' ? 'cost.history.hint' : 'cost.history.hint.day')}
				</Card.Description>
			</Card.Header>
			<Card.Content>
				<ChartPanel labels={history.labels} series={history.series} format={money} />
			</Card.Content>
		</Card.Root>
	{/if}
</div>
