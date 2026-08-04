<script lang="ts">
	import { api, type ModelRow, type Series } from '$lib/api';
	import ChartPanel from '$lib/components/chart-panel.svelte';
	import ResetView from '$lib/components/reset-view.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import X from '@lucide/svelte/icons/x';
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
	import { SUBSCRIPTIONS, billing, costOf, isPriced, plan } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';
	import { createTable } from '$lib/table.svelte';

	const COLUMNS = [
		{ id: 'model', label: 'cost.column.model' as const },
		{
			id: 'turns',
			label: 'cost.column.turns' as const,
			numeric: true,
			class: 'hidden @2xl:table-cell'
		},
		{
			id: 'sessions',
			label: 'cost.column.sessions' as const,
			numeric: true,
			class: 'hidden @3xl:table-cell'
		},
		{ id: 'inputTokens', label: 'cost.column.input' as const, numeric: true },
		{ id: 'outputTokens', label: 'cost.column.output' as const, numeric: true },
		{
			id: 'cacheReadTokens',
			label: 'cost.column.cacheRead' as const,
			numeric: true,
			class: 'hidden @2xl:table-cell'
		},
		{
			id: 'cacheWriteTokens',
			label: 'cost.column.cacheWrite' as const,
			numeric: true,
			class: 'hidden @3xl:table-cell'
		},
		{ id: 'cost', label: 'cost.column.cost' as const, numeric: true }
	];

	/** One hide rule per column, so a cell never outlives its header. */
	const CLASS: Record<string, string> = Object.fromEntries(
		COLUMNS.map((column) => [column.id, 'class' in column ? (column.class ?? '') : ''])
	);

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

	type Grain = 'month' | 'week' | 'day';
	/** What the history draws: money at API rates, or the tokens behind it. */
	type Measure = 'cost' | 'tokens';

	const GRAINS: Grain[] = ['month', 'week', 'day'];
	const MEASURES: Measure[] = ['cost', 'tokens'];

	let measure = $state<Measure>('cost');

	// Days and an area by default: the question is usually "what happened
	// lately", and a filled shape answers it faster than a line.
	let grain = $state<Grain>('day');

	const DAY_MS = 24 * 60 * 60 * 1000;

	/** The Monday of a date's week, which is the label a week bucket carries. */
	function weekOf(date: string): string {
		const at = Date.parse(`${date}T00:00:00Z`);
		// getUTCDay is 0 on Sunday; the week starts on Monday here.
		const offset = (new Date(at).getUTCDay() + 6) % 7;
		return new Date(at - offset * DAY_MS).toISOString().slice(0, 10);
	}

	/** Buckets the daily points, one series per model. */
	function bucket(by: Grain, measure: Measure = 'cost') {
		if (!series)
			return { labels: [] as string[], series: [] as { key: string; values: number[] }[] };

		const column = (date: string) =>
			by === 'month' ? date.slice(0, 7) : by === 'week' ? weekOf(date) : date;

		const points = series.points.filter((point) => !onlyModel || point.model === onlyModel);

		const labels: string[] = [];
		for (const point of points) {
			const label = column(point.date);
			if (!labels.includes(label)) labels.push(label);
		}
		labels.sort();

		const byKey: Record<string, number[]> = {};
		for (const point of points) {
			const index = labels.indexOf(column(point.date));
			if (index === -1) continue;
			byKey[point.key] ??= Array<number>(labels.length).fill(0);
			byKey[point.key][index] +=
				measure === 'cost'
					? costOf(asModel ?? point.model, point)
					: point.inputTokens + point.outputTokens + point.cacheReadTokens + point.cacheWriteTokens;
		}

		return {
			labels,
			series: Object.entries(byKey)
				.map(([key, values]) => ({ key: shortModel(key), values }))
				.sort((a, b) => b.values.reduce((x, y) => x + y, 0) - a.values.reduce((x, y) => x + y, 0))
		};
	}

	const history = $derived(bucket(grain, measure));
	/** The plan is billed by the month, so its card stays on months whatever the
	 *  chart above is set to. */
	const months = $derived(bucket('month'));

	const monthlyTotals = $derived(
		months.labels.map((_, index) =>
			months.series.reduce((sum, entry) => sum + (entry.values[index] ?? 0), 0)
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
		(rows ?? [])
			.filter((row) => !onlyModel || row.model === onlyModel)
			.map((row) => ({ row, cost: costOf(row.model, row) }))
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

	const totals = $derived(
		priced.reduce(
			(sum, { row }) => ({
				input: sum.input + row.inputTokens,
				output: sum.output + row.outputTokens,
				cacheRead: sum.cacheRead + row.cacheReadTokens,
				cacheWrite: sum.cacheWrite + row.cacheWriteTokens,
				turns: sum.turns + row.turns
			}),
			{ input: 0, output: 0, cacheRead: 0, cacheWrite: 0, turns: 0 }
		)
	);
	const totalTokens = $derived(totals.input + totals.output + totals.cacheRead + totals.cacheWrite);
	const totalTurns = $derived(totals.turns);

	/** The model the whole page is narrowed to, or null for all of them. */
	let onlyModel = $state<string | null>(null);

	function money(value: number): string {
		return region.format(value);
	}

	/** The API-rate cost against what each plan would have charged for it, so
	 *  the comparison is between plans and not only against the one in force. */
	const planLines = $derived([
		{ key: t('cost.plans.actual'), values: monthlyTotals.map((value) => value) },
		...SUBSCRIPTIONS.map((entry) => ({
			key: t(`cost.plan.${entry.id}`),
			values: months.labels.map(() => plan.monthlyOf(entry.id)),
			dashed: true
		}))
	]);

	function shortModel(model: string): string {
		return model.replace(/^claude-/, '');
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
		<!-- What was spent and what it was worth, each with the sentence that
		     makes the number mean something. A figure at API rates under a
		     subscription is the one that gets misread, so it says so itself. -->
		<div class="grid shrink-0 gap-2 @2xl:grid-cols-3">
			<div class="rounded-lg border bg-card p-3">
				<p class="text-xs text-muted-foreground">{t('cost.summary.tokens')}</p>
				<p class="text-2xl leading-tight font-semibold tabular-nums" title={exact(totalTokens)}>
					{compact(totalTokens)}
				</p>
				<p class="mt-1 text-xs text-muted-foreground">
					{t('cost.summary.tokens.hint', {
						input: compact(totals.input),
						output: compact(totals.output),
						cache: compact(totals.cacheRead + totals.cacheWrite)
					})}
				</p>
			</div>

			<div class="rounded-lg border bg-card p-3">
				<p class="text-xs text-muted-foreground">{t('cost.summary.cost')}</p>
				<p class="text-2xl leading-tight font-semibold tabular-nums">{money(totalCost)}</p>
				<p class="mt-1 text-xs text-muted-foreground">
					{t(
						billing.mode === 'subscription'
							? 'cost.summary.cost.subscription'
							: 'cost.summary.cost.api'
					)}
				</p>
			</div>

			<div class="rounded-lg border bg-card p-3">
				<p class="text-xs text-muted-foreground">{t('cost.summary.turns')}</p>
				<p class="text-2xl leading-tight font-semibold tabular-nums">{exact(totalTurns)}</p>
				<p class="mt-1 text-xs text-muted-foreground">
					{t('cost.summary.turns.hint', { models: exact(priced.length) })}
				</p>
			</div>
		</div>

		<div class="flex shrink-0 flex-wrap items-center gap-2">
			<Input placeholder={t('common.search')} class="h-8 max-w-xs" bind:value={table.query} />
			<ResetView show={table.dirty} onreset={() => table.reset()} />
			{#if onlyModel}
				<Button
					variant="secondary"
					size="sm"
					class="h-8 font-normal"
					onclick={() => (onlyModel = null)}
				>
					{shortModel(onlyModel)}
					<X class="size-3.5 opacity-60" />
				</Button>
			{/if}
			<span class="ml-auto text-xs text-muted-foreground tabular-nums">
				{t('cost.count', { count: exact(priced.length) })}
			</span>
		</div>

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
									<!-- The name is the control, not the row: a row that reacts
									     everywhere has no way to say what a click will do. -->
									<button
										type="button"
										class="rounded-sm underline-offset-4 hover:underline"
										title={t('cost.only', { model: shortModel(entry.row.model) })}
										onclick={() =>
											(onlyModel = onlyModel === entry.row.model ? null : entry.row.model)}
									>
										{shortModel(entry.row.model)}
									</button>
									{#if !isPriced(entry.row.model)}
										<Badge variant="outline" class="font-normal">{t('cost.unpriced')}</Badge>
									{/if}
								</div>
							</Table.Cell>
							<Table.Cell class={[CLASS.turns, 'text-right tabular-nums']}>
								{exact(entry.row.turns)}
							</Table.Cell>
							<Table.Cell class={[CLASS.sessions, 'text-right tabular-nums']}>
								{exact(entry.row.sessions)}
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums" title={exact(entry.row.inputTokens)}>
								{compact(entry.row.inputTokens)}
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums" title={exact(entry.row.outputTokens)}>
								{compact(entry.row.outputTokens)}
							</Table.Cell>
							<Table.Cell
								class={[CLASS.cacheReadTokens, 'text-right tabular-nums']}
								title={exact(entry.row.cacheReadTokens)}
							>
								{compact(entry.row.cacheReadTokens)}
							</Table.Cell>
							<Table.Cell
								class={[CLASS.cacheWriteTokens, 'text-right tabular-nums']}
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

		{#if history.labels.length > 0}
			<Card.Root data-size="sm" class="shrink-0">
				<Card.Header class="gap-1">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<Card.Title class="text-base">{t('cost.history')}</Card.Title>

						<div class="flex flex-wrap items-center gap-2">
							<div class="flex gap-1">
								{#each MEASURES as option (option)}
									<Button
										variant={measure === option ? 'default' : 'outline'}
										size="sm"
										class="h-8 font-normal"
										onclick={() => (measure = option)}
									>
										{t(option === 'cost' ? 'cost.measure.cost' : 'cost.measure.tokens')}
									</Button>
								{/each}
							</div>

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

							<!-- Repricing moves money, not tokens: the control would sit
							     there doing nothing while the chart shows counts. -->
							{#if measure === 'cost'}
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
							{/if}
						</div>
					</div>

					<Card.Description>
						{#if measure === 'tokens'}
							{t(grain === 'month' ? 'cost.history.tokens' : 'cost.history.tokens.day')}
						{:else}
							{t(grain === 'month' ? 'cost.history.hint' : 'cost.history.hint.day')}
						{/if}
						{#if measure === 'cost' && asModel && whatIfTotal !== null}
							· {t('cost.whatIf.note')} · {t('cost.whatIf.diff', {
								amount: `${whatIfTotal >= actualTotal ? '+' : '−'}${money(Math.abs(whatIfTotal - actualTotal))}`
							})}
						{/if}
					</Card.Description>
				</Card.Header>

				<Card.Content>
					<ChartPanel
						labels={history.labels}
						series={history.series}
						format={measure === 'cost' ? money : compact}
						type="area"
					/>
				</Card.Content>
			</Card.Root>

			<Card.Root data-size="sm" class="shrink-0">
				<Card.Header class="gap-2">
					<Card.Title class="text-base">{t('cost.plans.title')}</Card.Title>
					<Card.Description>
						{t('cost.plans.hint')}
					</Card.Description>
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

				<Card.Content class="flex flex-col gap-3 overflow-x-auto">
					<ChartPanel labels={months.labels} series={planLines} format={money} type="area" />

					<p class="text-xs text-muted-foreground">{t('cost.plans.description')}</p>

					<!-- A tariff that was recorded earlier still prices its months, and
					     can still be taken back; entering a new one belongs with the
					     tariff itself, in the settings. -->
					{#if plan.periods.length > 0}
						<div class="flex flex-wrap gap-1">
							{#each plan.periods as period (period.from)}
								<Badge variant="secondary" class="gap-1 font-normal">
									<span class="tabular-nums">{period.from}</span>
									<span>· {t(`cost.plan.${period.id}`)}</span>
									<button
										type="button"
										aria-label={t('cost.plans.remove')}
										class="ml-0.5 opacity-60 transition-opacity hover:opacity-100"
										onclick={() => plan.forget(period.from)}
									>
										×
									</button>
								</Badge>
							{/each}
						</div>
					{/if}
				</Card.Content>
			</Card.Root>
		{/if}

		<p class="shrink-0 text-xs text-muted-foreground">
			{#if region.isIndicative}
				{t('settings.rate.indicative.hint')}
			{/if}
			{t(billing.mode === 'subscription' ? 'cost.noteSubscription' : 'cost.note')}
		</p>
	{/if}
</div>
