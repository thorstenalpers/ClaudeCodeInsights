<script lang="ts">
	import { api, type RtkReport } from '$lib/api';
	import ChartPanel from '$lib/components/chart-panel.svelte';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import { compact, displayPath, exact, formatDate, formatWhen } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { rtk } from '$lib/rtk.svelte';

	/** As many days as the chart can carry without the axis turning into a smear. */
	const DAYS = 60;

	let report = $state<RtkReport | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	$effect(() => {
		if (!isHosted || !rtk.enabled) {
			loading = false;
			return;
		}

		let cancelled = false;
		error = null;
		api
			.getRtkReport()
			.then((value) => {
				if (!cancelled) report = value;
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

	/** The saved share of what the raw output would have cost. */
	function share(saved: number, input: number): string {
		return input > 0 ? `${Math.round((saved / input) * 100)}%` : '—';
	}

	function duration(ms: number): string {
		if (ms < 1000) return `${exact(ms)} ms`;
		if (ms < 60_000) return `${(ms / 1000).toFixed(1)} s`;
		return `${Math.round(ms / 60_000)} min`;
	}

	const summary = $derived(report?.summary ?? null);

	const cards = $derived(
		summary
			? [
					{
						id: 'saved',
						label: t('rtk.card.saved'),
						value: compact(summary.savedTokens),
						title: exact(summary.savedTokens),
						hint: t('rtk.hint.saved', { share: share(summary.savedTokens, summary.inputTokens) })
					},
					{
						id: 'input',
						label: t('rtk.card.input'),
						value: compact(summary.inputTokens),
						title: exact(summary.inputTokens),
						hint: t('rtk.hint.input')
					},
					{
						id: 'output',
						label: t('rtk.card.output'),
						value: compact(summary.outputTokens),
						title: exact(summary.outputTokens),
						hint: t('rtk.hint.output')
					},
					{
						id: 'commands',
						label: t('rtk.card.commands'),
						value: exact(summary.commands),
						title: exact(summary.commands),
						hint: t('rtk.hint.commands', { count: report?.filters.length ?? 0 })
					},
					{
						id: 'time',
						label: t('rtk.card.time'),
						value: duration(summary.execTimeMs),
						title: `${exact(summary.execTimeMs)} ms`,
						hint: t('rtk.hint.time')
					},
					{
						id: 'failures',
						label: t('rtk.card.failures'),
						value: exact(summary.failures),
						title: exact(summary.failures),
						hint: t('rtk.hint.failures')
					}
				]
			: []
	);

	const days = $derived(report?.days.slice(-DAYS) ?? []);
	const labels = $derived(days.map((day) => day.date));
	const series = $derived([
		{ key: t('rtk.series.kept'), values: days.map((day) => day.inputTokens - day.savedTokens) },
		{ key: t('rtk.series.saved'), values: days.map((day) => day.savedTokens) }
	]);

	/** Relative to the busiest row rather than to the total: with a long tail,
	 *  shares of the total are all invisible slivers. */
	const busiestFilter = $derived(report?.filters[0]?.savedTokens ?? 0);
	const busiestProject = $derived(report?.projects[0]?.savedTokens ?? 0);
</script>

<div class="@container flex flex-col gap-4 overflow-auto p-4">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if !rtk.enabled}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('rtk.off')}</Card.Title>
				<Card.Description>{t('rtk.off.hint')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('rtk.failed')}</Card.Title>
				<Card.Description class="font-mono text-xs">{error}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if loading && !report}
		<div class="grid grid-cols-1 gap-3 @xl:grid-cols-2 @3xl:grid-cols-3">
			{#each [...Array(6).keys()] as index (index)}
				<Skeleton class="h-28 w-full" />
			{/each}
		</div>
	{:else if summary && summary.commands === 0}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('rtk.empty')}</Card.Title>
				<Card.Description>
					{rtk.status?.historyExists
						? t('rtk.empty.hint')
						: t('rtk.empty.noHistory', { path: displayPath(rtk.status?.history ?? '') })}
				</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if report && summary}
		<div class="grid grid-cols-1 gap-3 @xl:grid-cols-2 @3xl:grid-cols-3">
			{#each cards as card (card.id)}
				<Card.Root data-size="sm">
					<Card.Header class="gap-0.5">
						<Card.Description>{card.label}</Card.Description>
						<Card.Title class="text-xl tabular-nums" title={card.title}>{card.value}</Card.Title>
						<p class="text-xs text-muted-foreground">{card.hint}</p>
					</Card.Header>
				</Card.Root>
			{/each}
		</div>

		<Card.Root data-size="sm">
			<Card.Header>
				<Card.Title>{t('rtk.daily')}</Card.Title>
				<Card.Description>{t('rtk.daily.description')}</Card.Description>
			</Card.Header>
			<Card.Content>
				<ChartPanel {labels} {series} format={compact} />
			</Card.Content>
		</Card.Root>

		<Card.Root data-size="sm">
			<Card.Header>
				<Card.Title>{t('rtk.filters')}</Card.Title>
				<Card.Description>{t('rtk.filters.description')}</Card.Description>
			</Card.Header>
			<Card.Content>
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>{t('rtk.column.filter')}</Table.Head>
							<Table.Head class="text-right">{t('rtk.column.calls')}</Table.Head>
							<Table.Head class="hidden text-right @md:table-cell">
								{t('rtk.column.input')}
							</Table.Head>
							<Table.Head class="text-right">{t('rtk.column.saved')}</Table.Head>
							<Table.Head class="text-right">{t('rtk.column.share')}</Table.Head>
							<Table.Head class="hidden w-40 @2xl:table-cell"></Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each report.filters as row (row.name)}
							<Table.Row>
								<Table.Cell class="font-medium">{row.name}</Table.Cell>
								<Table.Cell class="text-right tabular-nums">{exact(row.calls)}</Table.Cell>
								<Table.Cell class="hidden text-right tabular-nums @md:table-cell">
									{compact(row.inputTokens)}
								</Table.Cell>
								<Table.Cell class="text-right tabular-nums" title={exact(row.savedTokens)}>
									{compact(row.savedTokens)}
								</Table.Cell>
								<Table.Cell class="text-right tabular-nums">
									{share(row.savedTokens, row.inputTokens)}
								</Table.Cell>
								<Table.Cell class="hidden @2xl:table-cell">
									<div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
										<div
											class="h-full rounded-full bg-primary"
											style="width: {busiestFilter > 0
												? (row.savedTokens / busiestFilter) * 100
												: 0}%"
										></div>
									</div>
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			</Card.Content>
		</Card.Root>

		{#if report.projects.length > 0}
			<Card.Root data-size="sm">
				<Card.Header>
					<Card.Title>{t('rtk.projects')}</Card.Title>
					<Card.Description>{t('rtk.projects.description')}</Card.Description>
				</Card.Header>
				<Card.Content>
					<Table.Root>
						<Table.Header>
							<Table.Row>
								<Table.Head>{t('rtk.column.project')}</Table.Head>
								<Table.Head class="text-right">{t('rtk.column.calls')}</Table.Head>
								<Table.Head class="hidden text-right @md:table-cell">
									{t('rtk.column.input')}
								</Table.Head>
								<Table.Head class="text-right">{t('rtk.column.saved')}</Table.Head>
								<Table.Head class="text-right">{t('rtk.column.share')}</Table.Head>
								<Table.Head class="hidden w-40 @2xl:table-cell"></Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each report.projects as row (row.name)}
								<Table.Row>
									<Table.Cell class="max-w-64 truncate font-medium" title={displayPath(row.name)}>
										{row.name.split(/[\\/]/).filter(Boolean).at(-1) ?? row.name}
									</Table.Cell>
									<Table.Cell class="text-right tabular-nums">{exact(row.calls)}</Table.Cell>
									<Table.Cell class="hidden text-right tabular-nums @md:table-cell">
										{compact(row.inputTokens)}
									</Table.Cell>
									<Table.Cell class="text-right tabular-nums" title={exact(row.savedTokens)}>
										{compact(row.savedTokens)}
									</Table.Cell>
									<Table.Cell class="text-right tabular-nums">
										{share(row.savedTokens, row.inputTokens)}
									</Table.Cell>
									<Table.Cell class="hidden @2xl:table-cell">
										<div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
											<div
												class="h-full rounded-full bg-primary"
												style="width: {busiestProject > 0
													? (row.savedTokens / busiestProject) * 100
													: 0}%"
											></div>
										</div>
									</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				</Card.Content>
			</Card.Root>
		{/if}

		{#if report.failures.length > 0}
			<Card.Root data-size="sm">
				<Card.Header>
					<Card.Title>{t('rtk.failures')}</Card.Title>
					<Card.Description>{t('rtk.failures.description')}</Card.Description>
				</Card.Header>
				<Card.Content>
					<Table.Root>
						<Table.Header>
							<Table.Row>
								<Table.Head class="w-32">{t('rtk.column.when')}</Table.Head>
								<Table.Head>{t('rtk.column.command')}</Table.Head>
								<Table.Head class="hidden @xl:table-cell">{t('rtk.column.message')}</Table.Head>
								<Table.Head class="w-24 text-right">{t('rtk.column.recovered')}</Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each report.failures as row (`${row.ts}${row.command}`)}
								<Table.Row>
									<Table.Cell class="tabular-nums">{formatWhen(row.ts)}</Table.Cell>
									<Table.Cell class="max-w-64 truncate font-mono text-xs" title={row.command}>
										{row.command}
									</Table.Cell>
									<Table.Cell class="hidden max-w-64 truncate @xl:table-cell" title={row.message}>
										{row.message}
									</Table.Cell>
									<Table.Cell class="text-right">
										{row.recovered ? t('rtk.recovered') : t('rtk.lost')}
									</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				</Card.Content>
			</Card.Root>
		{/if}

		<p class="text-xs text-muted-foreground tabular-nums">
			{formatDate(summary.firstTs)} — {formatDate(summary.lastTs)}
		</p>
	{/if}
</div>
