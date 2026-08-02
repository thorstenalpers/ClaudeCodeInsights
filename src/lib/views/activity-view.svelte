<script lang="ts">
	import { api, type Rhythm } from '$lib/api';
	import { Badge } from '$lib/components/ui/badge';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { exact } from '$lib/format';
	import type { MessageKey } from '$lib/i18n/en';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { scan } from '$lib/scan.svelte';

	let rhythm = $state<Rhythm | null>(null);
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
			.getRhythm()
			.then((value) => {
				if (!cancelled) rhythm = value;
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

	const HOURS = [...Array(24).keys()];

	// Shading is relative to the busiest cell, so a quiet week still shows its
	// own shape instead of a uniformly pale grid.
	const peak = $derived(Math.max(1, ...(rhythm?.grid.flat() ?? [0])));

	function weekday(index: number): string {
		return t(`weekday.${index}` as MessageKey);
	}

	function intensity(count: number): number {
		return count === 0 ? 0 : 0.15 + (count / peak) * 0.85;
	}
</script>

<div class="flex h-full flex-col gap-4 overflow-auto p-6">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('overview.dbFailed')}</Card.Title>
				<Card.Description class="font-mono text-xs">{error}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if loading && !rhythm}
		<Skeleton class="h-64 w-full" />
	{:else if rhythm && rhythm.activeDays === 0}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('rhythm.empty')}</Card.Title>
				<Card.Description>{t('sessions.emptyScan')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if rhythm}
		<div class="flex shrink-0 flex-wrap items-center gap-2">
			{#if rhythm.busiestWeekday !== null && rhythm.busiestHour !== null}
				<Badge variant="secondary" class="font-normal">
					{t('rhythm.busiest', {
						weekday: weekday(rhythm.busiestWeekday),
						hour: t('rhythm.hour', { hour: rhythm.busiestHour })
					})}
				</Badge>
			{/if}
			<Badge variant="outline" class="font-normal">
				{t('rhythm.activeDays', { count: exact(rhythm.activeDays) })}
			</Badge>
			<Badge variant="outline" class="font-normal">
				{t('rhythm.longestStreak', { count: exact(rhythm.longestStreak) })}
			</Badge>
			{#if rhythm.currentStreak > 0}
				<Badge variant="outline" class="font-normal">
					{t('rhythm.currentStreak', { count: exact(rhythm.currentStreak) })}
				</Badge>
			{/if}
		</div>

		<div class="overflow-x-auto rounded-md border p-4">
			<div class="flex min-w-max flex-col gap-1">
				<div class="flex gap-1 pl-10">
					{#each HOURS as hour (hour)}
						<span class="w-5 text-center text-[10px] text-muted-foreground tabular-nums">
							{hour % 3 === 0 ? hour : ''}
						</span>
					{/each}
				</div>

				{#each rhythm.grid as row, day (day)}
					<div class="flex items-center gap-1">
						<span class="w-9 shrink-0 text-xs text-muted-foreground">{weekday(day)}</span>
						{#each row as count, hour (hour)}
							<Tooltip.Provider>
								<Tooltip.Root>
									<Tooltip.Trigger>
										{#snippet child({ props })}
											<div
												{...props}
												class="size-5 rounded-sm border border-transparent bg-primary"
												style="opacity: {intensity(count)}"
												class:bg-muted={count === 0}
											></div>
										{/snippet}
									</Tooltip.Trigger>
									<Tooltip.Content>
										{weekday(day)}
										{t('rhythm.hour', { hour })} · {exact(count)}
									</Tooltip.Content>
								</Tooltip.Root>
							</Tooltip.Provider>
						{/each}
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
