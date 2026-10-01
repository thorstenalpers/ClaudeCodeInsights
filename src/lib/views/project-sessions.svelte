<script lang="ts">
	/** The sessions of one project. The host filters, so the count under the
	 *  table is the whole project, not the page. */
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type SessionPage } from '$lib/api';
	import ActivityBadge from '$lib/components/activity-badge.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import { exact, formatWhen } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { nav } from '$lib/nav.svelte';
	import { costOf } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';

	type Props = { path: string };
	let { path }: Props = $props();

	const PAGE_SIZE = 25;

	let page = $state(0);
	let result = $state<SessionPage | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	$effect(() => {
		const query = {
			page,
			pageSize: PAGE_SIZE,
			sort: 'last',
			descending: true,
			projects: [path]
		};
		void scan.dataVersion;
		if (!isHosted) {
			loading = false;
			return;
		}

		let cancelled = false;
		error = null;
		api
			.listSessions(query)
			.then((value) => {
				if (!cancelled) result = value;
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

	const totalPages = $derived(result ? Math.max(1, Math.ceil(result.total / result.pageSize)) : 1);

	function open(sessionId: string, label: string) {
		nav.detailLabel = label;
		void goto(resolve('/sessions/[id]', { id: sessionId }));
	}
</script>

<div class="@container flex h-full flex-col gap-3 p-4">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('sessions.loadFailed')}</Card.Title>
				<Card.Description class="font-mono text-xs">{error}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if loading && !result}
		<div class="flex flex-col gap-2">
			{#each [...Array(6).keys()] as index (index)}
				<Skeleton class="h-10 w-full" />
			{/each}
		</div>
	{:else if result && result.rows.length === 0}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('sessions.emptyTitle')}</Card.Title>
				<Card.Description>{t('sessions.emptyScan')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if result}
		<span class="shrink-0 text-xs text-muted-foreground tabular-nums">
			{t('sessions.count', { count: exact(result.total) })}
		</span>

		<div
			class="min-h-0 flex-1 overflow-auto rounded-md border [&_td]:py-1 [&_td]:text-[13px] [&_th]:h-8 [&>[data-slot=table-container]]:overflow-visible"
		>
			<Table.Root>
				<Table.Header class="sticky top-0 z-10 bg-background">
					<Table.Row>
						<Table.Head>{t('sessions.column.topic')}</Table.Head>
						<Table.Head class="hidden @md:table-cell">{t('sessions.column.activity')}</Table.Head>
						<Table.Head>{t('sessions.column.last')}</Table.Head>
						<Table.Head class="text-right">{t('sessions.column.turns')}</Table.Head>
						<Table.Head class="text-right">{t('sessions.column.cost')}</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each result.rows as row (row.sessionId)}
						<Table.Row
							class="cursor-pointer"
							onclick={() => open(row.sessionId, row.topic ?? row.sessionId.slice(0, 8))}
						>
							<Table.Cell class="max-w-[20rem] truncate font-medium">
								{row.topic ?? row.sessionId.slice(0, 8)}
							</Table.Cell>
							<Table.Cell class="hidden @md:table-cell">
								<ActivityBadge activity={row.activity} profile={row.profile} />
							</Table.Cell>
							<Table.Cell class="whitespace-nowrap text-muted-foreground">
								{formatWhen(row.lastTs)}
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums">{row.turnCount}</Table.Cell>
							<Table.Cell class="text-right whitespace-nowrap tabular-nums">
								{row.model ? region.format(costOf(row.model, row)) : t('common.none')}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>

		<div class="flex shrink-0 items-center justify-between">
			<span class="text-xs text-muted-foreground tabular-nums">
				{t('sessions.page', { page: result.page + 1, total: totalPages })}
			</span>
			<div class="flex gap-1">
				<Button
					variant="outline"
					size="icon"
					class="size-7"
					disabled={result.page === 0}
					onclick={() => (page = Math.max(0, page - 1))}
					aria-label={t('sessions.previousPage')}
				>
					<ChevronLeft />
				</Button>
				<Button
					variant="outline"
					size="icon"
					class="size-7"
					disabled={result.page + 1 >= totalPages}
					onclick={() => (page = page + 1)}
					aria-label={t('sessions.nextPage')}
				>
					<ChevronRight />
				</Button>
			</div>
		</div>
	{/if}
</div>
