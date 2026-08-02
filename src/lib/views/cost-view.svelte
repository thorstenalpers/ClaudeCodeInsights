<script lang="ts">
	import { api, type ModelRow } from '$lib/api';
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
	import { billing, costOf, isPriced, uncachedCostOf, type BillingMode } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';
	import { createTable } from '$lib/table.svelte';

	const COLUMNS = [
		{ id: 'model', label: 'cost.column.model' as const },
		{ id: 'turns', label: 'cost.column.turns' as const, numeric: true },
		{ id: 'sessions', label: 'cost.column.sessions' as const, numeric: true },
		{ id: 'inputTokens', label: 'cost.column.input' as const, numeric: true },
		{ id: 'outputTokens', label: 'cost.column.output' as const, numeric: true },
		{ id: 'cacheReadTokens', label: 'cost.column.cacheRead' as const, numeric: true },
		{ id: 'cacheWriteTokens', label: 'cost.column.cacheWrite' as const, numeric: true },
		{ id: 'cost', label: 'cost.column.cost' as const, numeric: true }
	];

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

		<div
			class="min-h-0 flex-1 overflow-auto rounded-md border [&>[data-slot=table-container]]:overflow-visible"
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
								onsort={(id: string) => table.toggle(id)}
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
