<script lang="ts">
	import { api, type ModelRow } from '$lib/api';
	import { Badge } from '$lib/components/ui/badge';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import { compact, exact } from '$lib/format';
	import { i18n, t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { billing, costOf, isPriced, uncachedCostOf } from '$lib/pricing.svelte';
	import { scan } from '$lib/scan.svelte';

	let rows = $state<ModelRow[] | null>(null);
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

	const priced = $derived(
		(rows ?? []).map((row) => ({
			row,
			cost: costOf(row.model, row),
			uncached: uncachedCostOf(row.model, row)
		}))
	);

	const totalCost = $derived(priced.reduce((sum, entry) => sum + entry.cost, 0));
	const totalSaved = $derived(
		priced.reduce((sum, entry) => sum + Math.max(0, entry.uncached - entry.cost), 0)
	);

	function money(value: number): string {
		return new Intl.NumberFormat(i18n.intlLocale, {
			style: 'currency',
			currency: 'USD',
			maximumFractionDigits: value < 10 ? 2 : 0
		}).format(value);
	}

	function shortModel(model: string): string {
		return model.replace(/^claude-/, '');
	}
</script>

<div class="flex h-full flex-col gap-4 p-6">
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
			<span class="ml-auto text-xs text-muted-foreground tabular-nums">
				{t('cost.count', { count: exact(rows.length) })}
			</span>
		</div>

		<div
			class="min-h-0 flex-1 overflow-auto rounded-md border [&>[data-slot=table-container]]:overflow-visible"
		>
			<Table.Root>
				<Table.Header class="sticky top-0 z-10 bg-background">
					<Table.Row>
						<Table.Head>{t('cost.column.model')}</Table.Head>
						<Table.Head class="text-right">{t('cost.column.turns')}</Table.Head>
						<Table.Head class="text-right">{t('cost.column.sessions')}</Table.Head>
						<Table.Head class="text-right">{t('cost.column.input')}</Table.Head>
						<Table.Head class="text-right">{t('cost.column.output')}</Table.Head>
						<Table.Head class="text-right">{t('cost.column.cacheRead')}</Table.Head>
						<Table.Head class="text-right">{t('cost.column.cacheWrite')}</Table.Head>
						<Table.Head class="text-right">{t('cost.column.cost')}</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each priced as entry (entry.row.model)}
						<Table.Row>
							<Table.Cell class="font-medium">
								<div class="flex items-center gap-2">
									{shortModel(entry.row.model)}
									{#if !isPriced(entry.row.model)}
										<Badge variant="outline" class="font-normal">{t('cost.unpriced')}</Badge>
									{/if}
								</div>
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums">{exact(entry.row.turns)}</Table.Cell>
							<Table.Cell class="text-right tabular-nums">{exact(entry.row.sessions)}</Table.Cell>
							<Table.Cell class="text-right tabular-nums" title={exact(entry.row.inputTokens)}>
								{compact(entry.row.inputTokens)}
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums" title={exact(entry.row.outputTokens)}>
								{compact(entry.row.outputTokens)}
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums" title={exact(entry.row.cacheReadTokens)}>
								{compact(entry.row.cacheReadTokens)}
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums" title={exact(entry.row.cacheWriteTokens)}>
								{compact(entry.row.cacheWriteTokens)}
							</Table.Cell>
							<Table.Cell class="text-right font-medium tabular-nums">
								{isPriced(entry.row.model) ? money(entry.cost) : t('common.none')}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>

		<p class="shrink-0 text-xs text-muted-foreground">
			{t(billing.mode === 'subscription' ? 'cost.noteSubscription' : 'cost.note')}
		</p>
	{/if}
</div>
