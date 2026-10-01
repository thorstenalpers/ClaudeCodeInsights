<script lang="ts">
	import { api, type ModelRow, type Overview } from '$lib/api';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { compact, exact, formatDate } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { FAMILIES, billing, costOf, plan, rateFor } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';

	let overview = $state<Overview | null>(null);
	let models = $state<ModelRow[]>([]);
	let error = $state<string | null>(null);
	let loading = $state(true);

	async function load() {
		error = null;
		try {
			[overview, models] = await Promise.all([api.getOverview(), api.listModels()]);
		} catch (cause) {
			error = errorMessage(cause);
		} finally {
			loading = false;
		}
	}

	/** The same tokens priced at the published API rates, edits ignored. */
	function publishedCostOf(row: ModelRow): number {
		const family = FAMILIES.find((entry) => entry.match.test(row.model));
		const rate = family?.published ?? rateFor(row.model);
		return (
			(row.inputTokens * rate.input +
				row.outputTokens * rate.output +
				row.cacheReadTokens * rate.cacheRead +
				row.cacheWriteTokens * rate.cacheWrite) /
			1_000_000
		);
	}

	const apiCost = $derived(models.reduce((sum, row) => sum + publishedCostOf(row), 0));

	/** Months from the first turn to the last, for the subscription's fees.
	 *  Counted rather than stepped through with a Date, which keeps this a plain
	 *  calculation instead of a mutable object the linter has to police. */
	function monthsSpanned(): string[] {
		if (!overview?.firstTs || !overview.lastTs) return [];

		const [firstYear, firstMonth] = overview.firstTs.slice(0, 7).split('-').map(Number);
		const [lastYear, lastMonth] = overview.lastTs.slice(0, 7).split('-').map(Number);
		const span = (lastYear - firstYear) * 12 + (lastMonth - firstMonth);

		return Array.from({ length: Math.max(0, span) + 1 }, (_, index) => {
			const month = firstMonth - 1 + index;
			const year = firstYear + Math.floor(month / 12);
			return `${year}-${String((month % 12) + 1).padStart(2, '0')}`;
		});
	}

	// "My price": on a subscription the plan fees over the period, otherwise the
	// cost at the rate table as corrected — which is the API figure until edited.
	const myCost = $derived(
		billing.mode === 'subscription'
			? monthsSpanned().reduce((sum, month) => sum + plan.monthlyAt(month), 0)
			: models.reduce((sum, row) => sum + costOf(row.model, row), 0)
	);

	// Refetch when a scan lands new data, rather than polling.
	$effect(() => {
		void scan.dataVersion;
		if (isHosted) void load();
		else loading = false;
	});

	const costCards = $derived(
		overview && models.length > 0
			? [
					{
						id: 'costApi',
						label: t('overview.card.costApi'),
						value: region.format(apiCost),
						hint: t('overview.hint.costApi')
					},
					{
						id: 'costMine',
						label: t('overview.card.costMine'),
						value: region.format(myCost),
						hint: t(
							billing.mode === 'subscription' ? 'overview.hint.costPlan' : 'overview.hint.costRates'
						)
					}
				]
			: []
	);

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
		<div class="grid grid-cols-1 gap-3 @xl:grid-cols-2 @3xl:grid-cols-3">
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
		{#if costCards.length > 0}
			<div class="grid grid-cols-1 gap-3 @xl:grid-cols-2">
				{#each costCards as card (card.id)}
					<Card.Root data-size="sm">
						<Card.Header class="gap-0.5">
							<Card.Description>{card.label}</Card.Description>
							<Card.Title class="text-xl tabular-nums">{card.value}</Card.Title>
							<p class="text-xs text-muted-foreground">{card.hint}</p>
						</Card.Header>
					</Card.Root>
				{/each}
			</div>
		{/if}

		<div class="grid grid-cols-1 gap-3 @xl:grid-cols-2 @3xl:grid-cols-3">
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
