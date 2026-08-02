<script lang="ts">
	import { api, type AgentRow } from '$lib/api';
	import ResetView from '$lib/components/reset-view.svelte';
	import SortHeader from '$lib/components/sort-header.svelte';
	import * as Card from '$lib/components/ui/card';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import { compact, exact, formatWhen } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { costOfSplit } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';
	import { createTable } from '$lib/table.svelte';

	const COLUMNS = [
		{ id: 'agentType', label: 'agents.column.type' as const, info: 'info.runs' as const },
		{
			id: 'runs',
			label: 'agents.column.runs' as const,
			numeric: true,
			info: 'info.runs' as const
		},
		{
			id: 'totalTokens',
			label: 'agents.column.tokens' as const,
			numeric: true,
			info: 'info.tokens' as const
		},
		{
			id: 'cost',
			label: 'agents.column.cost' as const,
			numeric: true,
			info: 'info.agentCost' as const
		},
		{
			id: 'totalDurationMs',
			label: 'agents.column.duration' as const,
			numeric: true,
			info: 'info.duration' as const,
			class: 'hidden @md:table-cell'
		},
		{
			id: 'toolUseCount',
			label: 'agents.column.toolCalls' as const,
			numeric: true,
			info: 'info.calls' as const,
			class: 'hidden @xl:table-cell'
		},
		{
			id: 'lastTs',
			label: 'agents.column.last' as const,
			info: 'info.lastActive' as const,
			class: 'hidden @lg:table-cell'
		}
	];

	// One hide rule per column, read by both the header and the cell.
	const CLASS: Record<string, string> = Object.fromEntries(
		COLUMNS.map((column) => [column.id, 'class' in column ? (column.class ?? '') : ''])
	);

	let rows = $state<AgentRow[] | null>(null);
	let selected = $state<AgentRow | null>(null);
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
			.listAgents()
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

	const table = createTable<AgentRow>(
		() => rows ?? [],
		{
			agentType: (row) => row.agentType,
			runs: (row) => row.runs,
			totalTokens: (row) => row.totalTokens,
			cost: (row) => costOfSplit(row.byModel),
			totalDurationMs: (row) => row.totalDurationMs,
			toolUseCount: (row) => row.toolUseCount,
			lastTs: (row) => row.lastTs
		},
		{ sort: 'runs' }
	);

	function duration(ms: number): string {
		if (ms < 1000) return `${ms} ms`;
		const seconds = Math.round(ms / 1000);
		if (seconds < 60) return `${seconds}s`;
		const minutes = Math.floor(seconds / 60);
		return minutes < 60
			? `${minutes}m ${seconds % 60}s`
			: `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
	}
</script>

<div class="@container flex h-full flex-col gap-3 p-4">
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
				<Card.Title>{t('agents.empty')}</Card.Title>
				<Card.Description>{t('sessions.emptyScan')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if rows}
		<div class="flex shrink-0 flex-wrap items-center gap-2">
			<Input placeholder={t('common.search')} class="h-8 max-w-xs" bind:value={table.query} />
			<ResetView show={table.dirty} onreset={() => table.reset()} />
			<span class="ml-auto text-xs text-muted-foreground tabular-nums">
				{t('agents.count', { count: exact(rows.length) })}
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
								info={t(column.info)}
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
					{#each table.rows as row (row.agentType)}
						<Table.Row class="cursor-pointer" onclick={() => (selected = row)}>
							<Table.Cell class="font-medium">{row.agentType}</Table.Cell>
							<Table.Cell class="text-right tabular-nums">{exact(row.runs)}</Table.Cell>
							<Table.Cell class="text-right tabular-nums" title={exact(row.totalTokens)}>
								{compact(row.totalTokens)}
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums">
								{@const cost = costOfSplit(row.byModel)}
								{cost === null ? t('common.none') : region.format(cost)}
							</Table.Cell>
							<Table.Cell class={[CLASS.totalDurationMs, 'text-right tabular-nums']}>
								{duration(row.totalDurationMs)}
							</Table.Cell>
							<Table.Cell class={[CLASS.toolUseCount, 'text-right tabular-nums']}>
								{exact(row.toolUseCount)}
							</Table.Cell>
							<Table.Cell class={[CLASS.lastTs, 'whitespace-nowrap']}>
								{formatWhen(row.lastTs)}
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
</div>

<Dialog.Root
	open={selected !== null}
	onOpenChange={(open) => {
		if (!open) selected = null;
	}}
>
	<Dialog.Content class="max-w-lg">
		<Dialog.Header>
			<Dialog.Title>{selected?.agentType}</Dialog.Title>
			<Dialog.Description>{t('nav.agents.description')}</Dialog.Description>
		</Dialog.Header>

		{#if selected}
			{@const selectedCost = costOfSplit(selected.byModel)}
			<dl class="grid grid-cols-2 gap-3 text-sm">
				<dt class="text-muted-foreground">{t('agents.column.runs')}</dt>
				<dd class="text-right tabular-nums">{exact(selected.runs)}</dd>

				<dt class="text-muted-foreground">{t('agents.column.tokens')}</dt>
				<dd class="text-right tabular-nums">{exact(selected.totalTokens)}</dd>

				<dt class="text-muted-foreground">{t('agents.detail.perRun')}</dt>
				<dd class="text-right tabular-nums">
					{exact(Math.round(selected.totalTokens / Math.max(1, selected.runs)))}
				</dd>

				<dt class="text-muted-foreground">{t('agents.column.cost')}</dt>
				<dd class="text-right tabular-nums">
					{selectedCost === null ? t('common.none') : region.format(selectedCost)}
				</dd>

				<dt class="text-muted-foreground">{t('agents.column.duration')}</dt>
				<dd class="text-right tabular-nums">{duration(selected.totalDurationMs)}</dd>

				<dt class="text-muted-foreground">{t('agents.detail.avgDuration')}</dt>
				<dd class="text-right tabular-nums">
					{duration(Math.round(selected.totalDurationMs / Math.max(1, selected.runs)))}
				</dd>

				<dt class="text-muted-foreground">{t('agents.column.toolCalls')}</dt>
				<dd class="text-right tabular-nums">{exact(selected.toolUseCount)}</dd>

				<dt class="text-muted-foreground">{t('agents.detail.toolsPerRun')}</dt>
				<dd class="text-right tabular-nums">
					{(selected.toolUseCount / Math.max(1, selected.runs)).toFixed(1)}
				</dd>

				<dt class="text-muted-foreground">{t('agents.column.last')}</dt>
				<dd class="text-right">{formatWhen(selected.lastTs)}</dd>
			</dl>
		{/if}
	</Dialog.Content>
</Dialog.Root>
