<script lang="ts">
	import FileJson from '@lucide/svelte/icons/file-json';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Unlink from '@lucide/svelte/icons/unlink';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type ProjectRow, type ProjectsReport, type TranscriptFile } from '$lib/api';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import ResetView from '$lib/components/reset-view.svelte';
	import SortHeader from '$lib/components/sort-header.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import { Textarea } from '$lib/components/ui/textarea';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { compact, displayPath, exact, formatBytes, formatWhen } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { costOfSplit } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';
	import { createTable } from '$lib/table.svelte';

	const COLUMNS = [
		{ id: 'path', label: 'projects.column.project' as const, info: 'info.status' as const },
		{
			id: 'sessions',
			label: 'projects.column.sessions' as const,
			numeric: true,
			info: 'info.sessions' as const
		},
		{
			id: 'cost',
			label: 'projects.column.cost' as const,
			numeric: true,
			info: 'info.cost' as const
		},
		{
			id: 'turns',
			label: 'projects.column.turns' as const,
			numeric: true,
			info: 'info.turns' as const,
			class: 'hidden @lg:table-cell'
		},
		{
			id: 'inputTokens',
			label: 'projects.column.input' as const,
			numeric: true,
			info: 'info.input' as const,
			class: 'hidden @2xl:table-cell'
		},
		{
			id: 'outputTokens',
			label: 'projects.column.output' as const,
			numeric: true,
			info: 'info.output' as const,
			class: 'hidden @2xl:table-cell'
		},
		{
			id: 'transcriptBytes',
			label: 'projects.column.transcripts' as const,
			numeric: true,
			info: 'info.transcripts' as const,
			class: 'hidden @xl:table-cell'
		},
		{
			id: 'lastTs',
			label: 'projects.column.lastActive' as const,
			info: 'info.lastActive' as const,
			class: 'hidden @md:table-cell'
		}
	];

	// One hide rule per column, read by both the header and the cell.
	const CLASS: Record<string, string> = Object.fromEntries(
		COLUMNS.map((column) => [column.id, 'class' in column ? (column.class ?? '') : ''])
	);

	let report = $state<ProjectsReport | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	/** Bumped after every action that changes what the list would show. */
	let localVersion = $state(0);

	let deleteTarget = $state<ProjectRow | null>(null);
	let deleteFiles = $state<TranscriptFile[] | null>(null);
	let deleteBusy = $state(false);
	let deleteError = $state<string | null>(null);

	let removeTarget = $state<MergedRow | null>(null);
	let removeBusy = $state(false);
	let removeError = $state<string | null>(null);

	let settingsTarget = $state<ProjectRow | null>(null);
	let settingsText = $state('');
	let settingsBusy = $state(false);
	let settingsError = $state<string | null>(null);

	/** Shown after a write so the user knows where the safety copy went. */
	let lastBackup = $state<string | null>(null);

	$effect(() => {
		void scan.dataVersion;
		void localVersion;
		if (!isHosted) {
			loading = false;
			return;
		}

		let cancelled = false;
		error = null;
		api
			.listProjects()
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

	async function openDelete(project: ProjectRow) {
		deleteTarget = project;
		deleteFiles = null;
		deleteError = null;
		try {
			deleteFiles = await api.previewProjectTranscripts(project.path);
		} catch (cause) {
			deleteError = errorMessage(cause);
		}
	}

	async function confirmDelete() {
		if (!deleteTarget) return;
		deleteBusy = true;
		deleteError = null;
		try {
			const outcome = await api.deleteProjectTranscripts(deleteTarget.path);
			if (outcome.failed.length > 0) {
				deleteError = t('projects.delete.failed', { files: outcome.failed.join(', ') });
			} else {
				deleteTarget = null;
			}
			localVersion += 1;
		} catch (cause) {
			deleteError = errorMessage(cause);
		} finally {
			deleteBusy = false;
		}
	}

	async function confirmRemove() {
		if (!removeTarget) return;
		removeBusy = true;
		removeError = null;
		try {
			// Every spelling of the same directory, or the row would come back
			// with its twin still in the configuration.
			for (const path of removeTarget.spellings) {
				const outcome = await api.removeProjectRegistration(path);
				lastBackup = outcome.backupPath;
			}
			removeTarget = null;
			localVersion += 1;
		} catch (cause) {
			removeError = errorMessage(cause);
		} finally {
			removeBusy = false;
		}
	}

	async function openSettings(project: ProjectRow) {
		settingsTarget = project;
		settingsText = '';
		settingsError = null;
		try {
			settingsText = await api.getProjectSettings(project.path);
		} catch (cause) {
			settingsError = errorMessage(cause);
		}
	}

	async function saveSettings() {
		if (!settingsTarget) return;
		settingsBusy = true;
		settingsError = null;
		try {
			const outcome = await api.updateProjectSettings(settingsTarget.path, settingsText);
			lastBackup = outcome.backupPath;
			settingsTarget = null;
			localVersion += 1;
		} catch (cause) {
			settingsError = errorMessage(cause);
		} finally {
			settingsBusy = false;
		}
	}

	/** One project, with every spelling it is registered under. */
	type MergedRow = ProjectRow & { spellings: string[] };

	/**
	 * Two spellings of one directory are one project, and the list says so once.
	 *
	 * The figures are not added up: the history is keyed by the normalised path,
	 * so both registrations were already showing the same numbers. Only a second
	 * transcript folder — which only a difference in case produces — brings
	 * files of its own.
	 */
	const rows = $derived.by(() => {
		const merged: MergedRow[] = [];
		// A plain object, not a Map: this one is rebuilt from scratch on every
		// run and never observed, which is what the reactive Map is for.
		const byGroup: Record<string, MergedRow> = {};

		for (const project of report?.projects ?? []) {
			const group = project.duplicateGroup;
			const seen = group ? byGroup[group] : undefined;

			if (!seen) {
				const row: MergedRow = { ...project, spellings: [project.path] };
				if (group) byGroup[group] = row;
				merged.push(row);
				continue;
			}

			seen.spellings.push(project.path);
			seen.dirExists = seen.dirExists || project.dirExists;
			if (project.transcriptDir && project.transcriptDir !== seen.transcriptDir) {
				seen.transcriptFiles += project.transcriptFiles;
				seen.transcriptBytes += project.transcriptBytes;
			}
		}

		return merged;
	});

	/** What this one row needs doing, or nothing at all. */
	function adviceFor(row: ProjectRow): string {
		return [
			!row.registered && t('projects.advice.row.unregistered'),
			!row.dirExists && t('projects.advice.row.missingDir')
		]
			.filter(Boolean)
			.join(' ');
	}

	const table = createTable<MergedRow>(
		() => rows,
		{
			path: (row) => row.path,
			sessions: (row) => row.sessions,
			cost: (row) => costOfSplit(row.byModel),
			turns: (row) => row.turns,
			inputTokens: (row) => row.inputTokens,
			outputTokens: (row) => row.outputTokens,
			transcriptBytes: (row) => row.transcriptBytes,
			lastTs: (row) => row.lastTs
		},
		{ sort: 'lastTs' }
	);

	const deleteTotalBytes = $derived(
		deleteFiles?.reduce((sum, file) => sum + file.sizeBytes, 0) ?? 0
	);
</script>

<div class="@container flex h-full flex-col gap-3 p-4">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">
			{t('common.noHost')}
		</p>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('projects.loadFailed')}</Card.Title>
				<Card.Description class="font-mono text-xs">{error}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if loading && !report}
		<div class="flex flex-col gap-2">
			{#each [...Array(6).keys()] as index (index)}
				<Skeleton class="h-10 w-full" />
			{/each}
		</div>
	{:else if report}
		<div class="flex shrink-0 flex-wrap items-center gap-2">
			<Input placeholder={t('common.search')} class="h-8 max-w-xs" bind:value={table.query} />
			<ResetView show={table.dirty} onreset={() => table.reset()} />
			<Button
				variant="outline"
				size="sm"
				class="hidden h-8 max-w-full font-normal @xl:inline-flex"
				title={t('projects.source.open')}
				onclick={() => void api.openClaudeConfig()}
			>
				<FileJson class="size-3.5" />
				<span class="truncate"
					>{t('projects.source', { path: displayPath(report.configPath) })}</span
				>
			</Button>
			<span class="ml-auto text-xs text-muted-foreground tabular-nums">
				{t('projects.count', { count: exact(rows.length) })}
			</span>
		</div>

		{#if lastBackup}
			<p class="shrink-0 text-xs text-muted-foreground">
				{t('projects.backupWritten', { path: displayPath(lastBackup) })}
			</p>
		{/if}

		{#if report.projects.length === 0}
			<Card.Root>
				<Card.Header>
					<Card.Title>{t('projects.emptyTitle')}</Card.Title>
					<Card.Description>
						{report.configExists ? t('projects.emptyRegistered') : t('projects.emptyNoConfig')}
					</Card.Description>
				</Card.Header>
			</Card.Root>
		{:else}
			<div
				class="min-h-0 flex-1 overflow-auto rounded-md border [&_td]:py-1 [&_td]:text-[13px] [&_th]:h-8 [&>[data-slot=table-container]]:overflow-visible"
			>
				<Table.Root>
					<Table.Header class="sticky top-0 z-10 bg-background">
						<Table.Row>
							<SortHeader
								id="path"
								label={t('projects.column.project')}
								info={t('info.status')}
								direction={table.direction('path')}
								rank={table.rank('path')}
								multi={table.sorts.length > 1}
								kind={table.kind('path')}
								filtered={table.isFiltered('path')}
								options={table.options('path')}
								chosen={table.chosen('path')}
								text={table.textFilter('path')}
								range={table.range('path')}
								onsort={(id: string, additive: boolean) => table.toggle(id, additive)}
								ontoggle={(id: string, value: string) => table.toggleValue(id, value)}
								ontext={(id: string, value: string) => table.setText(id, value)}
								onrange={(id: string, bound: 'min' | 'max', value: string) =>
									table.setRange(id, bound, value)}
								onclear={(id: string) => table.clearFilter(id)}
							/>
							{#each COLUMNS.slice(1) as column (column.id)}
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
							<Table.Head class="w-28"></Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each table.rows as project (project.path + (project.transcriptDir ?? ''))}
							<Table.Row
								class="cursor-pointer"
								onclick={() =>
									void goto(
										resolve('/projects/[path]', { path: encodeURIComponent(project.path) })
									)}
							>
								<Table.Cell
									class="max-w-36 truncate font-mono text-xs @md:max-w-52 @2xl:max-w-72"
									title={displayPath(project.path)}
								>
									<span class="flex items-center gap-2">
										<span class="truncate">{displayPath(project.path)}</span>
										{#if adviceFor(project)}
											<Tooltip.Root>
												<Tooltip.Trigger>
													{#snippet child({ props })}
														<span {...props} class="shrink-0 text-amber-500">
															<TriangleAlert class="size-3.5" />
														</span>
													{/snippet}
												</Tooltip.Trigger>
												<Tooltip.Content class="max-w-72 text-xs font-normal">
													{adviceFor(project)}
												</Tooltip.Content>
											</Tooltip.Root>
										{/if}
									</span>
								</Table.Cell>
								<Table.Cell class="text-right tabular-nums">{exact(project.sessions)}</Table.Cell>
								<Table.Cell class="text-right tabular-nums">
									{@const cost = costOfSplit(project.byModel)}
									{cost === null ? t('common.none') : region.format(cost)}
								</Table.Cell>
								<Table.Cell class={[CLASS.turns, 'text-right tabular-nums']}>
									{exact(project.turns)}
								</Table.Cell>
								<Table.Cell class={[CLASS.inputTokens, 'text-right tabular-nums']}>
									{compact(project.inputTokens)}
								</Table.Cell>
								<Table.Cell class={[CLASS.outputTokens, 'text-right tabular-nums']}>
									{compact(project.outputTokens)}
								</Table.Cell>
								<Table.Cell class={[CLASS.transcriptBytes, 'text-right tabular-nums']}>
									{#if project.transcriptDir}
										{exact(project.transcriptFiles)} · {formatBytes(project.transcriptBytes)}
									{:else}
										—
									{/if}
								</Table.Cell>
								<Table.Cell class={[CLASS.lastTs, 'whitespace-nowrap']}>
									{formatWhen(project.lastTs)}
								</Table.Cell>
								<Table.Cell>
									<div
										class="flex justify-end gap-1"
										role="presentation"
										onclick={(event) => event.stopPropagation()}
									>
										{#if project.registered}
											<Tooltip.Provider>
												<Tooltip.Root>
													<Tooltip.Trigger>
														{#snippet child({ props })}
															<Button
																{...props}
																variant="ghost"
																size="icon"
																class="size-7"
																onclick={() => openSettings(project)}
															>
																<Pencil class="size-3.5" />
															</Button>
														{/snippet}
													</Tooltip.Trigger>
													<Tooltip.Content>{t('projects.action.editSettings')}</Tooltip.Content>
												</Tooltip.Root>
											</Tooltip.Provider>
											<Tooltip.Provider>
												<Tooltip.Root>
													<Tooltip.Trigger>
														{#snippet child({ props })}
															<Button
																{...props}
																variant="ghost"
																size="icon"
																class="size-7"
																onclick={() => (removeTarget = project)}
															>
																<Unlink class="size-3.5" />
															</Button>
														{/snippet}
													</Tooltip.Trigger>
													<Tooltip.Content
														>{t('projects.action.removeRegistration')}</Tooltip.Content
													>
												</Tooltip.Root>
											</Tooltip.Provider>
										{/if}
										{#if project.transcriptFiles > 0}
											<Tooltip.Provider>
												<Tooltip.Root>
													<Tooltip.Trigger>
														{#snippet child({ props })}
															<Button
																{...props}
																variant="ghost"
																size="icon"
																class="size-7 text-destructive"
																onclick={() => openDelete(project)}
															>
																<Trash2 class="size-3.5" />
															</Button>
														{/snippet}
													</Tooltip.Trigger>
													<Tooltip.Content>{t('projects.action.deleteTranscripts')}</Tooltip.Content
													>
												</Tooltip.Root>
											</Tooltip.Provider>
										{/if}
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
	{/if}
</div>

<AlertDialog.Root
	open={deleteTarget !== null}
	onOpenChange={(open) => {
		if (!open) deleteTarget = null;
	}}
>
	<AlertDialog.Content class="max-h-[calc(100dvh-2rem)] max-w-xl overflow-auto">
		<AlertDialog.Header>
			<AlertDialog.Title>{t('projects.delete.title')}</AlertDialog.Title>
			<AlertDialog.Description class="wrap-anywhere">
				{t('projects.delete.body', { path: displayPath(deleteTarget?.transcriptDir ?? '') })}
			</AlertDialog.Description>
		</AlertDialog.Header>

		{#if deleteFiles === null && !deleteError}
			<Skeleton class="h-24 w-full" />
		{:else if deleteFiles}
			<div
				class="max-h-64 overflow-auto rounded-md border [&_td]:py-1 [&_td]:text-[13px] [&_th]:h-8 [&>[data-slot=table-container]]:overflow-visible"
			>
				<Table.Root>
					<Table.Body>
						{#each deleteFiles as file (file.path)}
							<Table.Row>
								<Table.Cell class="py-1.5 font-mono text-xs">{file.name}</Table.Cell>
								<Table.Cell class="py-1.5 text-right text-xs tabular-nums">
									{formatBytes(file.sizeBytes)}
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>

				{#if deleteFiles.length === 0}
					<p class="p-4 text-sm text-muted-foreground">{t('projects.detail.noFiles')}</p>
				{/if}
			</div>
			<p class="text-xs text-muted-foreground tabular-nums">
				{t('projects.delete.summary', {
					count: exact(deleteFiles.length),
					size: formatBytes(deleteTotalBytes)
				})}
			</p>
		{/if}

		{#if deleteError}
			<p class="text-xs text-destructive">{deleteError}</p>
		{/if}

		<AlertDialog.Footer>
			<AlertDialog.Cancel disabled={deleteBusy}>{t('common.cancel')}</AlertDialog.Cancel>
			<Button variant="destructive" disabled={deleteBusy || !deleteFiles} onclick={confirmDelete}>
				{deleteBusy ? t('projects.delete.busy') : t('projects.delete.confirm')}
			</Button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

<AlertDialog.Root
	open={removeTarget !== null}
	onOpenChange={(open) => {
		if (!open) removeTarget = null;
	}}
>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{t('projects.remove.title')}</AlertDialog.Title>
			<AlertDialog.Description class="wrap-anywhere">
				{t('projects.remove.body', { path: displayPath(removeTarget?.path ?? '') })}
			</AlertDialog.Description>
		</AlertDialog.Header>

		{#if removeError}
			<p class="text-xs text-destructive">{removeError}</p>
		{/if}

		<AlertDialog.Footer>
			<AlertDialog.Cancel disabled={removeBusy}>{t('common.cancel')}</AlertDialog.Cancel>
			<Button variant="destructive" disabled={removeBusy} onclick={confirmRemove}>
				{removeBusy ? t('projects.remove.busy') : t('projects.remove.confirm')}
			</Button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

<Dialog.Root
	open={settingsTarget !== null}
	onOpenChange={(open) => {
		if (!open) settingsTarget = null;
	}}
>
	<Dialog.Content
		class="max-h-[calc(100dvh-2rem)] max-w-[calc(100%-2rem)] overflow-auto sm:max-w-2xl"
	>
		<Dialog.Header>
			<Dialog.Title>{t('projects.settings.title')}</Dialog.Title>
			<Dialog.Description class="wrap-anywhere">
				{t('projects.settings.body', { path: displayPath(settingsTarget?.path ?? '') })}
			</Dialog.Description>
		</Dialog.Header>

		<Textarea
			bind:value={settingsText}
			class="min-h-40 font-mono text-xs sm:min-h-72"
			spellcheck={false}
		/>

		{#if settingsError}
			<p class="text-xs text-destructive">{settingsError}</p>
		{/if}

		<Dialog.Footer>
			<Button variant="outline" disabled={settingsBusy} onclick={() => (settingsTarget = null)}>
				{t('common.cancel')}
			</Button>
			<Button disabled={settingsBusy || !settingsText} onclick={saveSettings}>
				{settingsBusy ? t('projects.settings.busy') : t('projects.settings.save')}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
