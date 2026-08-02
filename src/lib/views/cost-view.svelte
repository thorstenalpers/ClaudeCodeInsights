<script lang="ts">
	import { api, type ModelRow, type Series } from '$lib/api';
	import BarChart from '$lib/components/bar-chart.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Check from '@lucide/svelte/icons/check';
	import SortHeader from '$lib/components/sort-header.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { compact, exact } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import {
		PLANS,
		billing,
		costOf,
		isPriced,
		plan,
		uncachedCostOf,
		type BillingMode
	} from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';
	import { createTable } from '$lib/table.svelte';

	const COLUMNS = [
		{ id: 'model', label: 'cost.column.model' as const },
		{
			id: 'turns',
			label: 'cost.column.turns' as const,
			numeric: true,
			class: 'hidden @4xl:table-cell'
		},
		{
			id: 'sessions',
			label: 'cost.column.sessions' as const,
			numeric: true,
			class: 'hidden @5xl:table-cell'
		},
		{ id: 'inputTokens', label: 'cost.column.input' as const, numeric: true },
		{ id: 'outputTokens', label: 'cost.column.output' as const, numeric: true },
		{
			id: 'cacheReadTokens',
			label: 'cost.column.cacheRead' as const,
			numeric: true,
			class: 'hidden @4xl:table-cell'
		},
		{
			id: 'cacheWriteTokens',
			label: 'cost.column.cacheWrite' as const,
			numeric: true,
			class: 'hidden @5xl:table-cell'
		},
		{ id: 'cost', label: 'cost.column.cost' as const, numeric: true }
	];

	let rows = $state<ModelRow[] | null>(null);
	let series = $state<Series | null>(null);
	/** null means "as used": every model priced as itself. */
	let asModel = $state<string | null>(null);
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
			.listModels()
			.then((value) => {
				if (!cancelled) rows = value;
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
			.getSeries({ groupBy: 'model' })
			.then((value) => {
				if (!cancelled) series = value;
			})
			.catch(() => {
				// The table above already reports a failure; a second card saying
				// the same thing is noise.
			});

		return () => {
			cancelled = true;
		};
	});

	/** Buckets the daily points into months, one series per model. */
	const history = $derived.by(() => {
		if (!series)
			return { labels: [] as string[], series: [] as { key: string; values: number[] }[] };

		const months: string[] = [];
		for (const point of series.points) {
			const month = point.date.slice(0, 7);
			if (!months.includes(month)) months.push(month);
		}
		months.sort();

		const byKey: Record<string, number[]> = {};
		for (const point of series.points) {
			const column = months.indexOf(point.date.slice(0, 7));
			if (column === -1) continue;
			byKey[point.key] ??= Array<number>(months.length).fill(0);
			byKey[point.key][column] += costOf(asModel ?? point.model, point);
		}

		return {
			labels: months,
			series: Object.entries(byKey)
				.map(([key, values]) => ({ key: shortModel(key), values }))
				.sort((a, b) => b.values.reduce((x, y) => x + y, 0) - a.values.reduce((x, y) => x + y, 0))
		};
	});

	const monthlyTotals = $derived(
		history.labels.map((_, index) =>
			history.series.reduce((sum, entry) => sum + (entry.values[index] ?? 0), 0)
		)
	);

	/** What the plan has to beat: the average month at API rates. */
	const averageMonth = $derived(
		monthlyTotals.length > 0
			? monthlyTotals.reduce((sum, value) => sum + value, 0) / monthlyTotals.length
			: 0
	);

	const whatIfTotal = $derived.by(() => {
		const target = asModel;
		if (!series || !target) return null;
		return series.points.reduce((sum, point) => sum + costOf(target, point), 0);
	});

	const actualTotal = $derived(
		series ? series.points.reduce((sum, point) => sum + costOf(point.model, point), 0) : 0
	);

	const priced = $derived(
		(rows ?? []).map((row) => ({
			row,
			cost: costOf(row.model, row),
			uncached: uncachedCostOf(row.model, row)
		}))
	);

	const table = createTable<(typeof priced)[number]>(
		() => priced,
		{
			model: (entry) => entry.row.model,
			turns: (entry) => entry.row.turns,
			sessions: (entry) => entry.row.sessions,
			inputTokens: (entry) => entry.row.inputTokens,
			outputTokens: (entry) => entry.row.outputTokens,
			cacheReadTokens: (entry) => entry.row.cacheReadTokens,
			cacheWriteTokens: (entry) => entry.row.cacheWriteTokens,
			cost: (entry) => entry.cost
		},
		{ sort: 'cost' }
	);

	const totalCost = $derived(priced.reduce((sum, entry) => sum + entry.cost, 0));
	const totalSaved = $derived(
		priced.reduce((sum, entry) => sum + Math.max(0, entry.uncached - entry.cost), 0)
	);

	function money(value: number): string {
		return region.format(value);
	}

	const BILLING: BillingMode[] = ['api', 'subscription'];

	function shortModel(model: string): string {
		return model.replace(/^claude-/, '');
	}
</script>

<div class="flex h-full flex-col gap-3 overflow-auto p-4">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('overview.dbFailed')}</Card.Title>
				<Card.Description class="font-mono text-xs">{error}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if loading && !rows}
		<div class="flex flex-col gap-2">
			{#each [...Array(5).keys()] as index (index)}
				<Skeleton class="h-10 w-full" />
			{/each}
		</div>
	{:else if rows && rows.length === 0}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('cost.empty')}</Card.Title>
				<Card.Description>{t('sessions.emptyScan')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if rows}
		<div class="flex shrink-0 flex-wrap items-center gap-3">
			<Input placeholder={t('common.search')} class="max-w-48" bind:value={table.query} />
			<span class="text-lg font-semibold tabular-nums">
				{t(billing.mode === 'subscription' ? 'cost.totalEquivalent' : 'cost.total', {
					amount: money(totalCost)
				})}
			</span>
			{#if totalSaved > 0}
				<Badge variant="secondary" class="font-normal">
					{t('cost.saved', { amount: money(totalSaved) })}
				</Badge>
			{/if}
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="outline" size="sm">
							{t(`settings.billing.${billing.mode}`)}
							<ChevronDown class="size-3.5 opacity-60" />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content align="start" class="w-64">
					{#each BILLING as option (option)}
						<DropdownMenu.Item onSelect={() => billing.set(option)}>
							<span class="flex flex-1 flex-col gap-0.5">
								<span>{t(`settings.billing.${option}`)}</span>
								<span class="text-xs text-muted-foreground">
									{t(`settings.billing.${option}Hint`)}
								</span>
							</span>
							{#if billing.mode === option}
								<Check class="size-4" />
							{/if}
						</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Content>
			</DropdownMenu.Root>

			<span class="ml-auto text-xs text-muted-foreground tabular-nums">
				{t('cost.count', { count: exact(rows.length) })}
			</span>
		</div>

		{#if history.labels.length > 0}
			<Card.Root data-size="sm" class="shrink-0">
				<Card.Header class="gap-1">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<Card.Title class="text-base">{t('cost.history')}</Card.Title>

						<div class="flex items-center gap-2">
							<span class="text-xs text-muted-foreground">{t('cost.whatIf')}</span>
							<DropdownMenu.Root>
								<DropdownMenu.Trigger>
									{#snippet child({ props })}
										<Button {...props} variant="outline" size="sm" class="h-8 font-normal">
											{asModel ? shortModel(asModel) : t('cost.whatIf.actual')}
											<ChevronDown class="size-3.5 opacity-60" />
										</Button>
									{/snippet}
								</DropdownMenu.Trigger>
								<DropdownMenu.Content align="end" class="w-56">
									<DropdownMenu.Item onSelect={() => (asModel = null)}>
										<span class="flex-1">{t('cost.whatIf.actual')}</span>
										{#if asModel === null}
											<Check class="size-4" />
										{/if}
									</DropdownMenu.Item>
									<DropdownMenu.Separator />
									{#each rows ?? [] as row (row.model)}
										<DropdownMenu.Item onSelect={() => (asModel = row.model)}>
											<span class="flex-1 truncate">{shortModel(row.model)}</span>
											{#if asModel === row.model}
												<Check class="size-4" />
											{/if}
										</DropdownMenu.Item>
									{/each}
								</DropdownMenu.Content>
							</DropdownMenu.Root>
						</div>
					</div>

					{#if asModel && whatIfTotal !== null}
						<Card.Description>
							{t('cost.whatIf.note')} · {t('cost.whatIf.diff', {
								amount: `${whatIfTotal >= actualTotal ? '+' : '−'}${money(Math.abs(whatIfTotal - actualTotal))}`
							})}
						</Card.Description>
					{/if}
				</Card.Header>

				<Card.Content>
					<BarChart labels={history.labels} series={history.series} format={money} />
				</Card.Content>
			</Card.Root>

			<Card.Root data-size="sm" class="shrink-0">
				<Card.Header class="gap-2">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<Card.Title class="text-base">{t('cost.plan')}</Card.Title>
						<div class="flex flex-wrap gap-1">
							{#each PLANS as entry (entry.id)}
								<Button
									variant={plan.id === entry.id ? 'default' : 'outline'}
									size="sm"
									class="h-8 font-normal"
									onclick={() => plan.set(entry.id)}
								>
									{t(`cost.plan.${entry.id}`)}
								</Button>
							{/each}
						</div>
					</div>
					<Card.Description>
						{t('cost.plan.monthly', { amount: money(plan.monthly) })} ·
						{averageMonth >= plan.monthly
							? t('cost.plan.verdict.worth', { amount: money(averageMonth) })
							: t('cost.plan.verdict.under', {
									amount: money(averageMonth),
									plan: t(`cost.plan.${plan.id}`)
								})}
					</Card.Description>
				</Card.Header>

				<Card.Content class="overflow-x-auto">
					<Table.Root>
						<Table.Header>
							<Table.Row>
								<Table.Head>{t('cost.month')}</Table.Head>
								<Table.Head class="text-right">{t('cost.column.cost')}</Table.Head>
								<Table.Head class="text-right">{t('cost.plan')}</Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each history.labels as month, index (month)}
								<Table.Row>
									<Table.Cell class="font-medium tabular-nums">{month}</Table.Cell>
									<Table.Cell class="text-right tabular-nums">
										{money(monthlyTotals[index])}
									</Table.Cell>
									<Table.Cell class="text-right text-muted-foreground tabular-nums">
										{money(plan.monthly)}
									</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				</Card.Content>
			</Card.Root>
		{/if}

		<div
			class="min-h-0 flex-1 overflow-auto rounded-md border [&_td]:py-1 [&_td]:text-[13px] [&_th]:h-8 [&>[data-slot=table-container]]:overflow-visible"
		>
			<Table.Root>
				<Table.Header class="sticky top-0 z-10 bg-background">
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
					{#each table.rows as entry (entry.row.model)}
						<Table.Row>
							<Table.Cell class="font-medium">
								<div class="flex items-center gap-2">
									{shortModel(entry.row.model)}
									{#if !isPriced(entry.row.model)}
										<Badge variant="outline" class="font-normal">{t('cost.unpriced')}</Badge>
									{/if}
								</div>
							</Table.Cell>
							<Table.Cell class="hidden text-right tabular-nums lg:table-cell"
								>{exact(entry.row.turns)}</Table.Cell
							>
							<Table.Cell class="hidden text-right tabular-nums xl:table-cell"
								>{exact(entry.row.sessions)}</Table.Cell
							>
							<Table.Cell class="text-right tabular-nums" title={exact(entry.row.inputTokens)}>
								{compact(entry.row.inputTokens)}
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums" title={exact(entry.row.outputTokens)}>
								{compact(entry.row.outputTokens)}
							</Table.Cell>
							<Table.Cell
								class="hidden text-right tabular-nums lg:table-cell"
								title={exact(entry.row.cacheReadTokens)}
							>
								{compact(entry.row.cacheReadTokens)}
							</Table.Cell>
							<Table.Cell
								class="hidden text-right tabular-nums xl:table-cell"
								title={exact(entry.row.cacheWriteTokens)}
							>
								{compact(entry.row.cacheWriteTokens)}
							</Table.Cell>
							<Table.Cell class="text-right font-medium tabular-nums">
								{isPriced(entry.row.model) ? money(entry.cost) : t('common.none')}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>

			{#if table.rows.length === 0}
				<p class="p-4 text-sm text-muted-foreground">{t('common.noMatch')}</p>
			{/if}
		</div>

		<p class="shrink-0 text-xs text-muted-foreground">
			{#if region.needsRate}
				{t('settings.rate.missing')}
			{/if}
			{t(billing.mode === 'subscription' ? 'cost.noteSubscription' : 'cost.note')}
		</p>
	{/if}
</div>
