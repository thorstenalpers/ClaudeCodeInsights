<script lang="ts">
	import { api, type ToolRow } from '$lib/api';
	import SortHeader from '$lib/components/sort-header.svelte';
	import * as Card from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import { exact } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { scan } from '$lib/scan.svelte';
	import { createTable } from '$lib/table.svelte';

	const COLUMNS = [
		{ id: 'name', label: 'tools.column.name' as const },
		{ id: 'calls', label: 'tools.column.calls' as const, numeric: true },
		{
			id: 'sessions',
			label: 'tools.column.sessions' as const,
			numeric: true,
			class: 'hidden @2xl:table-cell'
		}
	];

	let rows = $state<ToolRow[] | null>(null);
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
			.listTools()
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

	const table = createTable<ToolRow>(
		() => rows ?? [],
		{
			name: (row) => row.name,
			calls: (row) => row.calls,
			sessions: (row) => row.sessions
		},
		{ sort: 'calls' }
	);

	// The bar is relative to the busiest tool rather than to the total: with a
	// long tail, shares against the total are all invisible slivers.
	const busiest = $derived(rows?.[0]?.calls ?? 0);
	const totalCalls = $derived(rows?.reduce((sum, row) => sum + row.calls, 0) ?? 0);
</script>

<div class="flex h-full flex-col gap-3 p-4">
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
			{#each [...Array(8).keys()] as index (index)}
				<Skeleton class="h-10 w-full" />
			{/each}
		</div>
	{:else if rows && rows.length === 0}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('tools.empty')}</Card.Title>
				<Card.Description>{t('sessions.emptyScan')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if rows}
		<div class="flex shrink-0 flex-wrap items-center gap-2">
			<Input placeholder={t('common.search')} class="max-w-xs" bind:value={table.query} />
			<span class="text-xs text-muted-foreground tabular-nums">
				{t('tools.count', { count: exact(rows.length) })}
			</span>
			<span class="ml-auto text-xs text-muted-foreground tabular-nums">
				{t('tools.column.calls')}: {exact(totalCalls)}
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
						<Table.Head class="hidden w-64 lg:table-cell">{t('tools.column.share')}</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each table.rows as row (row.name)}
						<Table.Row>
							<Table.Cell class="font-medium">{row.name}</Table.Cell>
							<Table.Cell class="text-right tabular-nums">{exact(row.calls)}</Table.Cell>
							<Table.Cell class="hidden text-right tabular-nums md:table-cell"
								>{exact(row.sessions)}</Table.Cell
							>
							<Table.Cell class="hidden @4xl:table-cell">
								<div class="flex items-center gap-2">
									<div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
										<div
											class="h-full rounded-full bg-primary"
											style="width: {busiest > 0 ? (row.calls / busiest) * 100 : 0}%"
										></div>
									</div>
									<span class="w-12 shrink-0 text-right text-xs text-muted-foreground tabular-nums">
										{totalCalls > 0 ? Math.round((row.calls / totalCalls) * 100) : 0}%
									</span>
								</div>
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
