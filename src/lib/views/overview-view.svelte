<script lang="ts">
	import { api, type Overview } from '$lib/api';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { compact, exact, formatDate } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { scan } from '$lib/scan.svelte';

	let overview = $state<Overview | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	async function load() {
		error = null;
		try {
			overview = await api.getOverview();
		} catch (cause) {
			error = errorMessage(cause);
		} finally {
			loading = false;
		}
	}

	// Refetch when a scan lands new data, rather than polling.
	$effect(() => {
		void scan.dataVersion;
		if (isHosted) void load();
		else loading = false;
	});

	const cards = $derived(
		overview
			? [
					{
						id: 'sessions',
						label: t('overview.card.sessions'),
						value: overview.sessions,
						hint: t('overview.hint.activeDays', { count: overview.activeDays })
					},
					{
						id: 'turns',
						label: t('overview.card.turns'),
						value: overview.turns,
						hint: t('overview.hint.turns')
					},
					{
						id: 'input',
						label: t('overview.card.input'),
						value: overview.inputTokens,
						hint: t('overview.hint.input')
					},
					{
						id: 'output',
						label: t('overview.card.output'),
						value: overview.outputTokens,
						hint: t('overview.hint.output')
					},
					{
						id: 'cacheRead',
						label: t('overview.card.cacheRead'),
						value: overview.cacheReadTokens,
						hint: t('overview.hint.cacheRead')
					},
					{
						id: 'cacheWrite',
						label: t('overview.card.cacheWrite'),
						value: overview.cacheWriteTokens,
						hint: t('overview.hint.cacheWrite')
					}
				]
			: []
	);
</script>

<div class="@container flex flex-col gap-4 overflow-auto p-4">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if loading}
		<div class="grid grid-cols-1 gap-3 @2xl:grid-cols-2 @5xl:grid-cols-3">
			{#each [...Array(6).keys()] as index (index)}
				<Skeleton class="h-28 w-full" />
			{/each}
		</div>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('overview.dbFailed')}</Card.Title>
				<Card.Description class="font-mono text-xs">{error}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if overview && overview.turns === 0}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('overview.emptyTitle')}</Card.Title>
				<Card.Description>{t('overview.emptyBody')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if overview}
		<div class="grid grid-cols-1 gap-3 @2xl:grid-cols-2 @5xl:grid-cols-3">
			{#each cards as card (card.id)}
				<Card.Root data-size="sm">
					<Card.Header class="gap-0.5">
						<Card.Description>{card.label}</Card.Description>
						<Card.Title class="text-xl tabular-nums" title={exact(card.value)}>
							{compact(card.value)}
						</Card.Title>
						<p class="text-xs text-muted-foreground">{card.hint}</p>
					</Card.Header>
				</Card.Root>
			{/each}
		</div>

		<p class="text-xs text-muted-foreground tabular-nums">
			{formatDate(overview.firstTs)} — {formatDate(overview.lastTs)}
		</p>
	{/if}
</div>
