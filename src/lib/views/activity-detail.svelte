<script lang="ts">
	/**
	 * One activity, and the work that carries its name.
	 *
	 * The totals come from the same roll-up the table above uses; the history
	 * and the sessions are the host's own queries narrowed to this one label, so
	 * nothing here is a second reading of the same numbers.
	 */
	import { api, type ActivityRow, type SessionPage, type Series } from '$lib/api';
	import ChartPanel from '$lib/components/chart-panel.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import { compact, exact, formatWhen } from '$lib/format';
	import type { MessageKey } from '$lib/i18n/en';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { FAMILIES, costOf, costOfSplit, rates } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';

	type Props = { activity: string };
	let { activity }: Props = $props();

	type Grain = 'month' | 'day';
	const GRAINS: Grain[] = ['month', 'day'];

	let grain = $state<Grain>('day');
	let row = $state<ActivityRow | null>(null);
	let series = $state<Series | null>(null);
	let sessions = $state<SessionPage | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	$effect(() => {
		const wanted = activity;
		void scan.dataVersion;
		if (!isHosted) {
			loading = false;
			return;
		}

		let cancelled = false;
		error = null;
		Promise.all([
			api.listActivities(),
			api.getSeries({ groupBy: 'model', activities: [wanted] }),
			api.listSessions({
				activities: [wanted],
				pageSize: 25,
				sort: 'last',
				descending: true,
				rates: FAMILIES.map((family) => ({ family: family.id, ...rates.for(family.id) }))
			})
		])
			.then(([all, points, page]) => {
				if (cancelled) return;
				row = all.find((entry) => entry.activity === wanted) ?? null;
				series = points;
				sessions = page;
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

	const label = $derived.by(() => {
		const key = `activity.${activity}` as MessageKey;
		const named = t(key);
		return named === key ? activity : named;
	});

	const history = $derived.by(() => {
		if (!series)
			return { labels: [] as string[], series: [] as { key: string; values: number[] }[] };

		const column = (date: string) => (grain === 'month' ? date.slice(0, 7) : date);

		const labels: string[] = [];
		for (const point of series.points) {
			const at = column(point.date);
			if (!labels.includes(at)) labels.push(at);
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

	const cost = $derived(row ? costOfSplit(row.byModel) : null);

	function money(value: number): string {
		return region.format(value);
	}

	function openSession(id: string): void {
		void goto(resolve('/sessions/[id]', { id }));
	}
</script>

<div class="@container flex h-full flex-col gap-3 overflow-auto p-4">
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
	{:else if !row}
		<Card.Root>
			<Card.Header>
				<Card.Title>{label}</Card.Title>
				<Card.Description>{t('sessions.emptyScan')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else}
		<div class="flex shrink-0 flex-wrap items-center gap-2">
			<Badge variant="secondary" class="font-normal">
				{t('cost.column.sessions')}: {exact(row.sessions)}
			</Badge>
			<Badge variant="outline" class="font-normal">
				{t('cost.column.turns')}: {exact(row.turns)}
			</Badge>
			<Badge variant="outline" class="font-normal">
				{t('nav.projects')}: {exact(row.projects)}
			</Badge>
			<Badge variant="outline" class="font-normal">
				{t('cost.column.input')}: {compact(row.inputTokens)}
			</Badge>
			<Badge variant="outline" class="font-normal">
				{t('cost.column.output')}: {compact(row.outputTokens)}
			</Badge>
			<Badge variant="outline" class="font-normal">
				{t('sessions.column.cache')}: {compact(row.cacheReadTokens + row.cacheWriteTokens)}
			</Badge>
			<Badge variant="outline" class="font-normal">
				{t('cost.column.cost')}: {cost === null ? t('common.none') : money(cost)}
			</Badge>
			<span class="ml-auto text-xs text-muted-foreground">
				{formatWhen(row.lastTs)}
			</span>
		</div>

		{#if history.labels.length > 0}
			<Card.Root data-size="sm" class="shrink-0">
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
					<ChartPanel labels={history.labels} series={history.series} format={money} type="area" />
				</Card.Content>
			</Card.Root>
		{/if}

		<Card.Root data-size="sm" class="min-h-0 shrink-0">
			<Card.Header class="gap-1">
				<Card.Title class="text-base">{t('nav.sessions')}</Card.Title>
				<Card.Description>
					{t('activity.detail.sessions', {
						shown: exact(sessions?.rows.length ?? 0),
						total: exact(sessions?.total ?? 0)
					})}
				</Card.Description>
			</Card.Header>
			<Card.Content class="overflow-auto [&_td]:py-1 [&_td]:text-[13px] [&_th]:h-8">
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>{t('sessions.column.topic')}</Table.Head>
							<Table.Head class="hidden @xl:table-cell">
								{t('sessions.column.project')}
							</Table.Head>
							<Table.Head class="text-right">{t('sessions.column.turns')}</Table.Head>
							<Table.Head class="hidden text-right @md:table-cell">
								{t('sessions.column.cost')}
							</Table.Head>
							<Table.Head class="whitespace-nowrap">{t('sessions.column.last')}</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each sessions?.rows ?? [] as session (session.sessionId)}
							<Table.Row class="cursor-pointer" onclick={() => openSession(session.sessionId)}>
								<Table.Cell class="max-w-72 truncate font-medium">
									{session.topic ?? session.sessionId.slice(0, 8)}
								</Table.Cell>
								<Table.Cell class="hidden max-w-48 truncate @xl:table-cell">
									{session.projectName ?? ''}
								</Table.Cell>
								<Table.Cell class="text-right tabular-nums">
									{exact(session.turnCount)}
								</Table.Cell>
								<Table.Cell class="hidden text-right tabular-nums @md:table-cell">
									{money(costOf(session.model ?? '', session))}
								</Table.Cell>
								<Table.Cell class="whitespace-nowrap text-muted-foreground">
									{formatWhen(session.lastTs)}
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>

				{#if (sessions?.rows.length ?? 0) === 0}
					<p class="p-4 text-sm text-muted-foreground">{t('common.noMatch')}</p>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
</div>
