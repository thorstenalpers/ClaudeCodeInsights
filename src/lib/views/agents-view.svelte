<script lang="ts">
	import { api, type AgentRow } from '$lib/api';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import { compact, exact, formatWhen } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { scan } from '$lib/scan.svelte';

	let rows = $state<AgentRow[] | null>(null);
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
				<Card.Title>{t('agents.empty')}</Card.Title>
				<Card.Description>{t('sessions.emptyScan')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if rows}
		<div class="flex shrink-0 items-center">
			<span class="ml-auto text-xs text-muted-foreground tabular-nums">
				{t('agents.count', { count: exact(rows.length) })}
			</span>
		</div>

		<div
			class="min-h-0 flex-1 overflow-auto rounded-md border [&>[data-slot=table-container]]:overflow-visible"
		>
			<Table.Root>
				<Table.Header class="sticky top-0 z-10 bg-background">
					<Table.Row>
						<Table.Head>{t('agents.column.type')}</Table.Head>
						<Table.Head class="text-right">{t('agents.column.runs')}</Table.Head>
						<Table.Head class="text-right">{t('agents.column.tokens')}</Table.Head>
						<Table.Head class="text-right">{t('agents.column.duration')}</Table.Head>
						<Table.Head class="text-right">{t('agents.column.toolCalls')}</Table.Head>
						<Table.Head>{t('agents.column.last')}</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each rows as row (row.agentType)}
						<Table.Row>
							<Table.Cell class="font-medium">{row.agentType}</Table.Cell>
							<Table.Cell class="text-right tabular-nums">{exact(row.runs)}</Table.Cell>
							<Table.Cell class="text-right tabular-nums" title={exact(row.totalTokens)}>
								{compact(row.totalTokens)}
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums">
								{duration(row.totalDurationMs)}
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums">{exact(row.toolUseCount)}</Table.Cell>
							<Table.Cell class="whitespace-nowrap">{formatWhen(row.lastTs)}</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>
	{/if}
</div>
