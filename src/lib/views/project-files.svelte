<script lang="ts">
	/** The transcript files behind the figures, named and sized — the same list
	 *  the delete dialog shows before it removes anything. */
	import { api, type TranscriptFile } from '$lib/api';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import { exact, formatBytes } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { scan } from '$lib/scan.svelte';

	type Props = { path: string };
	let { path }: Props = $props();

	let files = $state<TranscriptFile[] | null>(null);
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
			.previewProjectTranscripts(wanted)
			.then((value) => {
				if (!cancelled) files = value;
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

	const totalBytes = $derived(files?.reduce((sum, file) => sum + file.sizeBytes, 0) ?? 0);
</script>

<div class="flex h-full flex-col gap-3 p-4">
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
		<div class="flex flex-col gap-2">
			{#each [...Array(6).keys()] as index (index)}
				<Skeleton class="h-8 w-full" />
			{/each}
		</div>
	{:else if !files || files.length === 0}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('projects.detail.noFiles')}</Card.Title>
			</Card.Header>
		</Card.Root>
	{:else}
		<span class="shrink-0 text-xs text-muted-foreground tabular-nums">
			{t('projects.detail.fileCount', {
				count: exact(files.length),
				size: formatBytes(totalBytes)
			})}
		</span>

		<div
			class="min-h-0 flex-1 overflow-auto rounded-md border [&_td]:py-1 [&_td]:text-[13px] [&_th]:h-8 [&>[data-slot=table-container]]:overflow-visible"
		>
			<Table.Root>
				<Table.Header class="sticky top-0 z-10 bg-background">
					<Table.Row>
						<Table.Head>{t('projects.detail.file')}</Table.Head>
						<Table.Head class="text-right">{t('projects.column.transcripts')}</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each files as file (file.path)}
						<Table.Row>
							<Table.Cell class="font-mono text-xs">{file.name}</Table.Cell>
							<Table.Cell class="text-right tabular-nums">
								{formatBytes(file.sizeBytes)}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>
	{/if}
</div>
