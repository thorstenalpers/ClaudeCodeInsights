<script lang="ts">
	import { api, type ProjectRow } from '$lib/api';
	import { Badge } from '$lib/components/ui/badge';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { compact, displayPath, exact, formatBytes, formatWhen } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { costOfSplit } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';

	type Props = { path: string };
	let { path }: Props = $props();

	let row = $state<ProjectRow | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	$effect(() => {
		const wanted = path;
		void scan.dataVersion;
		if (!isHosted) {
			loading = false;
			return;
		}

		let cancelled = false;
		error = null;
		api
			.listProjects()
			.then((report) => {
				if (!cancelled) row = report.projects.find((entry) => entry.path === wanted) ?? null;
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

	const cost = $derived(row ? costOfSplit(row.byModel) : null);

	const figures = $derived(
		row
			? [
					{ id: 'sessions', label: t('projects.column.sessions'), value: exact(row.sessions) },
					{ id: 'turns', label: t('projects.column.turns'), value: exact(row.turns) },
					{
						id: 'cost',
						label: t('projects.column.cost'),
						value: cost === null ? t('common.none') : region.format(cost)
					},
					{ id: 'input', label: t('projects.column.input'), value: compact(row.inputTokens) },
					{ id: 'output', label: t('projects.column.output'), value: compact(row.outputTokens) },
					{
						id: 'cache',
						label: t('cost.column.cacheRead'),
						value: compact(row.cacheReadTokens)
					}
				]
			: []
	);
</script>

<div class="@container flex flex-col gap-4 p-4">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('projects.loadFailed')}</Card.Title>
				<Card.Description class="font-mono text-xs">{error}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if loading}
		<div class="grid grid-cols-1 gap-3 @xl:grid-cols-2 @3xl:grid-cols-3">
			{#each [...Array(6).keys()] as index (index)}
				<Skeleton class="h-24 w-full" />
			{/each}
		</div>
	{:else if !row}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('projects.detail.gone')}</Card.Title>
				<Card.Description class="font-mono text-xs">{displayPath(path)}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else}
		<div class="flex flex-wrap items-center gap-2">
			{#if !row.registered}
				<Badge variant="outline">{t('projects.badge.unregistered')}</Badge>
			{/if}
			{#if !row.dirExists}
				<Badge variant="destructive">{t('projects.badge.missingDir')}</Badge>
			{/if}
			<span class="text-xs text-muted-foreground">
				{t('projects.column.lastActive')}: {formatWhen(row.lastTs)}
			</span>
		</div>

		<div class="grid grid-cols-1 gap-3 @xl:grid-cols-2 @3xl:grid-cols-3">
			{#each figures as figure (figure.id)}
				<Card.Root data-size="sm">
					<Card.Header class="gap-0.5">
						<Card.Description>{figure.label}</Card.Description>
						<Card.Title class="text-xl tabular-nums">{figure.value}</Card.Title>
					</Card.Header>
				</Card.Root>
			{/each}
		</div>

		<Card.Root data-size="sm">
			<Card.Header class="gap-1">
				<Card.Title class="text-base">{t('projects.detail.files')}</Card.Title>
				<Card.Description class="font-mono text-xs break-all">
					{row.transcriptDir ? displayPath(row.transcriptDir) : t('common.none')}
				</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="text-sm tabular-nums">
					{t('projects.detail.fileCount', {
						count: exact(row.transcriptFiles),
						size: formatBytes(row.transcriptBytes)
					})}
				</p>
			</Card.Content>
		</Card.Root>
	{/if}
</div>
