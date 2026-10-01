<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type ActivityRow, type Rhythm, type Series } from '$lib/api';
	import ChartPanel from '$lib/components/chart-panel.svelte';
	import { Button } from '$lib/components/ui/button';
	import ResetView from '$lib/components/reset-view.svelte';
	import SortHeader from '$lib/components/sort-header.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import * as Card from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { compact, exact, formatWhen } from '$lib/format';
	import type { MessageKey } from '$lib/i18n/en';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { costOfSplit } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';
	import { createTable } from '$lib/table.svelte';

	let rhythm = $state<Rhythm | null>(null);
	let series = $state<Series | null>(null);
	let rows = $state<ActivityRow[] | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	$effect(() => {
		void scan.dataVersion;
		if (!isHosted) {
			loading = false;
			return;
		}

		let cancelled = false;
		error = null;
		api
			.getSeries({ groupBy: 'activity' })
			.then((value) => (series = value))
			.catch(() => (series = null));

		api
			.getRhythm()
			.then((value) => {
				if (!cancelled) rhythm = value;
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

	$effect(() => {
		void scan.dataVersion;
		if (!isHosted) return;

		let cancelled = false;
		api
			.listActivities()
			.then((value) => {
				if (!cancelled) rows = value;
			})
			.catch(() => {
				// The grid below already reports a failure of the same database.
			});

		return () => {
			cancelled = true;
		};
	});

	const COLUMNS = [
		{ id: 'activity', label: 'sessions.column.activity' as const },
		{ id: 'sessions', label: 'cost.column.sessions' as const, numeric: true },
		{ id: 'turns', label: 'cost.column.turns' as const, numeric: true },
		{
			id: 'projects',
			label: 'nav.projects' as const,
			numeric: true,
			class: 'hidden @md:table-cell'
		},
		{
			id: 'inputTokens',
			label: 'cost.column.input' as const,
			numeric: true,
			class: 'hidden @xl:table-cell'
		},
		{
			id: 'outputTokens',
			label: 'cost.column.output' as const,
			numeric: true,
			class: 'hidden @xl:table-cell'
		},
		{
			id: 'cacheTokens',
			label: 'sessions.column.cache' as const,
			numeric: true,
			class: 'hidden @3xl:table-cell'
		},
		{
			id: 'lastTs',
			label: 'sessions.column.last' as const,
			class: 'hidden @2xl:table-cell'
		},
		{ id: 'cost', label: 'cost.column.cost' as const, numeric: true }
	];

	const CLASS: Record<string, string> = Object.fromEntries(
		COLUMNS.map((column) => [column.id, 'class' in column ? (column.class ?? '') : ''])
	);

	function label(activity: string): string {
		const key = `activity.${activity}` as MessageKey;
		const named = t(key);
		return named === key ? activity : named;
	}

	const cacheOf = (row: ActivityRow) => row.cacheReadTokens + row.cacheWriteTokens;

	/** Opens the activity's own page. */
	function openActivity(activity: string): void {
		void goto(resolve('/activity/[name]', { name: activity }));
	}

	const table = createTable<ActivityRow>(
		() => rows ?? [],
		{
			// Sorted and filtered on the name a reader sees, not on the id behind it.
			activity: (row) => label(row.activity),
			sessions: (row) => row.sessions,
			turns: (row) => row.turns,
			projects: (row) => row.projects,
			inputTokens: (row) => row.inputTokens,
			outputTokens: (row) => row.outputTokens,
			cacheTokens: cacheOf,
			lastTs: (row) => row.lastTs ?? '',
			cost: (row) => costOfSplit(row.byModel)
		},
		{ sort: 'sessions' }
	);

	/** What the whole set adds up to, which is what a reader wants first. */
	const totals = $derived(
		(rows ?? []).reduce(
			(sum, row) => ({
				sessions: sum.sessions + row.sessions,
				turns: sum.turns + row.turns,
				tokens:
					sum.tokens +
					row.inputTokens +
					row.outputTokens +
					row.cacheReadTokens +
					row.cacheWriteTokens,
				// Null where no model is known, which is not the same as free.
				cost: sum.cost + (costOfSplit(row.byModel) ?? 0)
			}),
			{ sessions: 0, turns: 0, tokens: 0, cost: 0 }
		)
	);

	/** The busiest activity, named rather than left to be read off a bar. */
	const busiest = $derived([...(rows ?? [])].sort((a, b) => b.sessions - a.sessions)[0] ?? null);

	type Grain = 'month' | 'week' | 'day';
	const GRAINS: Grain[] = ['month', 'week', 'day'];
	let grain = $state<Grain>('day');

	const DAY_MS = 24 * 60 * 60 * 1000;

	function weekOf(date: string): string {
		const at = Date.parse(`${date}T00:00:00Z`);
		const offset = (new Date(at).getUTCDay() + 6) % 7;
		return new Date(at - offset * DAY_MS).toISOString().slice(0, 10);
	}

	/** Turns per activity over time, bucketed the way the reader asked. */
	const history = $derived.by(() => {
		const points = series?.points ?? [];
		if (points.length === 0)
			return { labels: [] as string[], series: [] as { key: string; values: number[] }[] };

		const column = (date: string) =>
			grain === 'month' ? date.slice(0, 7) : grain === 'week' ? weekOf(date) : date;

		const labels: string[] = [];
		for (const point of points) {
			const at = column(point.date);
			if (!labels.includes(at)) labels.push(at);
		}
		labels.sort();

		const byKey: Record<string, number[]> = {};
		for (const point of points) {
			const index = labels.indexOf(column(point.date));
			if (index === -1) continue;
			byKey[point.key] ??= Array<number>(labels.length).fill(0);
			byKey[point.key][index] += point.turns;
		}

		return {
			labels,
			series: Object.entries(byKey)
				.map(([key, values]) => ({ key: label(key), values }))
				.sort((a, b) => b.values.reduce((x, y) => x + y, 0) - a.values.reduce((x, y) => x + y, 0))
		};
	});

	const HOURS = [...Array(24).keys()];

	// Shading is relative to the busiest cell, so a quiet week still shows its
	// own shape instead of a uniformly pale grid.
	const peak = $derived(Math.max(1, ...(rhythm?.grid.flat() ?? [0])));

	function weekday(index: number): string {
		return t(`weekday.${index}` as MessageKey);
	}

	function intensity(count: number): number {
		return count === 0 ? 0 : 0.15 + (count / peak) * 0.85;
	}
</script>

<div class="flex h-full flex-col gap-4 overflow-auto p-6">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('overview.dbFailed')}</Card.Title>
				<Card.Description class="font-mono text-xs">{error}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if loading && !rhythm}
		<Skeleton class="h-64 w-full" />
	{:else if rhythm && rhythm.activeDays === 0}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('rhythm.empty')}</Card.Title>
				<Card.Description>{t('sessions.emptyScan')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if rhythm}
		<div class="flex shrink-0 flex-wrap items-center gap-2">
			{#if rhythm.busiestWeekday !== null && rhythm.busiestHour !== null}
				<Badge variant="secondary" class="font-normal">
					{t('rhythm.busiest', {
						weekday: weekday(rhythm.busiestWeekday),
						hour: t('rhythm.hour', { hour: rhythm.busiestHour })
					})}
				</Badge>
			{/if}
			<Badge variant="outline" class="font-normal">
				{t('rhythm.activeDays', { count: exact(rhythm.activeDays) })}
			</Badge>
			<Badge variant="outline" class="font-normal">
				{t('rhythm.longestStreak', { count: exact(rhythm.longestStreak) })}
			</Badge>
			{#if rhythm.currentStreak > 0}
				<Badge variant="outline" class="font-normal">
					{t('rhythm.currentStreak', { count: exact(rhythm.currentStreak) })}
				</Badge>
			{/if}
		</div>

		{#if rows && rows.length > 0}
			<div class="grid shrink-0 gap-2 @2xl:grid-cols-4">
				<div class="rounded-lg border bg-card p-3">
					<p class="text-xs text-muted-foreground">{t('cost.column.sessions')}</p>
					<p class="text-2xl leading-tight font-semibold tabular-nums">
						{exact(totals.sessions)}
					</p>
					{#if busiest}
						<p class="mt-1 truncate text-xs text-muted-foreground">
							{t('activity.busiest', { activity: label(busiest.activity) })}
						</p>
					{/if}
				</div>
				<div class="rounded-lg border bg-card p-3">
					<p class="text-xs text-muted-foreground">{t('cost.column.turns')}</p>
					<p class="text-2xl leading-tight font-semibold tabular-nums">{exact(totals.turns)}</p>
					<p class="mt-1 text-xs text-muted-foreground">
						{t('activity.perSession', {
							turns: totals.sessions === 0 ? '0' : Math.round(totals.turns / totals.sessions)
						})}
					</p>
				</div>
				<div class="rounded-lg border bg-card p-3">
					<p class="text-xs text-muted-foreground">{t('cost.summary.tokens')}</p>
					<p class="text-2xl leading-tight font-semibold tabular-nums" title={exact(totals.tokens)}>
						{compact(totals.tokens)}
					</p>
					<p class="mt-1 text-xs text-muted-foreground">{t('cost.summary.cost')}</p>
				</div>
				<div class="rounded-lg border bg-card p-3">
					<p class="text-xs text-muted-foreground">{t('cost.column.cost')}</p>
					<p class="text-2xl leading-tight font-semibold tabular-nums">
						{region.format(totals.cost)}
					</p>
					<p class="mt-1 text-xs text-muted-foreground">{t('cost.summary.cost.api')}</p>
				</div>
			</div>

			{#if history.labels.length > 0}
				<Card.Root data-size="sm" class="shrink-0">
					<Card.Header class="gap-1">
						<div class="flex flex-wrap items-center justify-between gap-2">
							<Card.Title class="text-base">{t('activity.history')}</Card.Title>
							<div class="flex gap-1">
								{#each GRAINS as option (option)}
									<Button
										variant={grain === option ? 'default' : 'outline'}
										size="sm"
										class="h-8 font-normal"
										onclick={() => (grain = option)}
									>
										{t(
											option === 'month'
												? 'cost.byMonth'
												: option === 'week'
													? 'cost.byWeek'
													: 'cost.byDay'
										)}
									</Button>
								{/each}
							</div>
						</div>
						<Card.Description>{t('activity.history.hint')}</Card.Description>
					</Card.Header>
					<Card.Content>
						<ChartPanel
							labels={history.labels}
							series={history.series}
							format={compact}
							type="stacked"
						/>
					</Card.Content>
				</Card.Root>
			{/if}

			<div class="flex shrink-0 flex-wrap items-center gap-2">
				<Input placeholder={t('common.search')} class="h-8 max-w-xs" bind:value={table.query} />
				<ResetView show={table.dirty} onreset={() => table.reset()} />
			</div>

			<div
				class="shrink-0 overflow-auto rounded-md border [&_td]:py-1 [&_td]:text-[13px] [&_th]:h-8 [&>[data-slot=table-container]]:overflow-visible"
			>
				<Table.Root>
					<Table.Header class="bg-background">
						<Table.Row>
							{#each COLUMNS as column (column.id)}
								<SortHeader
									{...column}
									label={t(column.label)}
									direction={table.direction(column.id)}
									rank={table.rank(column.id)}
									multi={table.sorts.length > 1}
									kind={table.kind(column.id)}
									filtered={table.isFiltered(column.id)}
									options={table.options(column.id)}
									chosen={table.chosen(column.id)}
									text={table.textFilter(column.id)}
									range={table.range(column.id)}
									onsort={(id: string, additive: boolean) => table.toggle(id, additive)}
									ontoggle={(id: string, value: string) => table.toggleValue(id, value)}
									ontext={(id: string, value: string) => table.setText(id, value)}
									onrange={(id: string, bound: 'min' | 'max', value: string) =>
										table.setRange(id, bound, value)}
									onclear={(id: string) => table.clearFilter(id)}
								/>
							{/each}
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each table.rows as row (row.activity)}
							<!-- Into the sessions behind the number: the sessions page
							     takes the filter from the URL. -->
							<Table.Row class="cursor-pointer" onclick={() => openActivity(row.activity)}>
								<Table.Cell class="font-medium">{label(row.activity)}</Table.Cell>
								<Table.Cell class="text-right tabular-nums">{exact(row.sessions)}</Table.Cell>
								<Table.Cell class="text-right tabular-nums">{exact(row.turns)}</Table.Cell>
								<Table.Cell class={[CLASS.projects, 'text-right tabular-nums']}>
									{exact(row.projects)}
								</Table.Cell>
								<Table.Cell class={[CLASS.inputTokens, 'text-right tabular-nums']}>
									{compact(row.inputTokens)}
								</Table.Cell>
								<Table.Cell class={[CLASS.outputTokens, 'text-right tabular-nums']}>
									{compact(row.outputTokens)}
								</Table.Cell>
								<Table.Cell class={[CLASS.cacheTokens, 'text-right tabular-nums']}>
									{compact(cacheOf(row))}
								</Table.Cell>
								<Table.Cell class={[CLASS.lastTs, 'whitespace-nowrap text-muted-foreground']}>
									{formatWhen(row.lastTs)}
								</Table.Cell>
								<Table.Cell class="text-right tabular-nums">
									{@const cost = costOfSplit(row.byModel)}
									{cost === null ? t('common.none') : region.format(cost)}
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>

				{#if table.rows.length === 0}
					<p class="p-4 text-sm text-muted-foreground">{t('common.noMatch')}</p>
				{/if}
			</div>
		{/if}

		<div class="overflow-x-auto rounded-md border p-4">
			<div class="flex min-w-max flex-col gap-1">
				<div class="flex gap-1 pl-10">
					{#each HOURS as hour (hour)}
						<span class="w-5 text-center text-[10px] text-muted-foreground tabular-nums">
							{hour % 3 === 0 ? hour : ''}
						</span>
					{/each}
				</div>

				{#each rhythm.grid as row, day (day)}
					<div class="flex items-center gap-1">
						<span class="w-9 shrink-0 text-xs text-muted-foreground">{weekday(day)}</span>
						{#each row as count, hour (hour)}
							<Tooltip.Provider>
								<Tooltip.Root>
									<Tooltip.Trigger>
										{#snippet child({ props })}
											<div
												{...props}
												class="size-5 rounded-sm border border-transparent bg-primary"
												style="opacity: {intensity(count)}"
												class:bg-muted={count === 0}
											></div>
										{/snippet}
									</Tooltip.Trigger>
									<Tooltip.Content>
										{weekday(day)}
										{t('rhythm.hour', { hour })} · {exact(count)}
									</Tooltip.Content>
								</Tooltip.Root>
							</Tooltip.Provider>
						{/each}
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
