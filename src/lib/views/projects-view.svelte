<script lang="ts">
	import Pencil from '@lucide/svelte/icons/pencil';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Unlink from '@lucide/svelte/icons/unlink';
	import { api, type ProjectRow, type ProjectsReport, type TranscriptFile } from '$lib/api';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Badge } from '$lib/components/ui/badge';
	import SortHeader from '$lib/components/sort-header.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import { Textarea } from '$lib/components/ui/textarea';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { compact, exact, formatBytes, formatWhen } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { scan } from '$lib/scan.svelte';
	import { createTable } from '$lib/table.svelte';

	const COLUMNS = [
		{ id: 'path', label: 'projects.column.project' as const },
		{ id: 'status', label: 'projects.column.status' as const },
		{ id: 'sessions', label: 'projects.column.sessions' as const, numeric: true },
		{
			id: 'turns',
			label: 'projects.column.turns' as const,
			numeric: true,
			class: 'hidden @2xl:table-cell'
		},
		{
			id: 'inputTokens',
			label: 'projects.column.input' as const,
			numeric: true,
			class: 'hidden @3xl:table-cell'
		},
		{
			id: 'outputTokens',
			label: 'projects.column.output' as const,
			numeric: true,
			class: 'hidden @3xl:table-cell'
		},
		{
			id: 'transcriptBytes',
			label: 'projects.column.transcripts' as const,
			numeric: true,
			class: 'hidden @xl:table-cell'
		},
		{ id: 'lastTs', label: 'projects.column.lastActive' as const, class: 'hidden @xl:table-cell' }
	];

	let report = $state<ProjectsReport | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	/** Bumped after every action that changes what the list would show. */
	let localVersion = $state(0);

	let deleteTarget = $state<ProjectRow | null>(null);
	let deleteFiles = $state<TranscriptFile[] | null>(null);
	let deleteBusy = $state(false);
	let deleteError = $state<string | null>(null);

	let removeTarget = $state<ProjectRow | null>(null);
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
			const outcome = await api.removeProjectRegistration(removeTarget.path);
			lastBackup = outcome.backupPath;
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

	/** The badges as one sortable, filterable value. */
	function statusOf(row: ProjectRow): string {
		const flags = [
			!row.registered && t('projects.badge.unregistered'),
			!row.dirExists && t('projects.badge.missingDir'),
			row.duplicateGroup && t('projects.badge.duplicate')
		].filter(Boolean);
		return flags.length > 0 ? flags.join(', ') : t('projects.badge.ok');
	}

	const table = createTable<ProjectRow>(
		() => report?.projects ?? [],
		{
			path: (row) => row.path,
			status: (row) => statusOf(row),
			sessions: (row) => row.sessions,
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

<div class="flex h-full flex-col gap-3 p-4">
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
			<Input placeholder={t('common.search')} class="max-w-xs" bind:value={table.query} />
			<span class="text-xs text-muted-foreground">
				{t('projects.source', { path: report.configPath })}
			</span>
			<span class="ml-auto text-xs text-muted-foreground tabular-nums">
				{t('projects.count', { count: exact(report.projects.length) })}
			</span>
		</div>

		{#if lastBackup}
			<p class="shrink-0 text-xs text-muted-foreground">
				{t('projects.backupWritten', { path: lastBackup })}
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
							<Table.Row>
								<Table.Cell class="max-w-96 truncate font-mono text-xs" title={project.path}>
									{project.path}
								</Table.Cell>
								<Table.Cell>
									<div class="flex flex-wrap gap-1">
										{#if !project.registered}
											<Badge variant="outline">{t('projects.badge.unregistered')}</Badge>
										{/if}
										{#if !project.dirExists}
											<Badge variant="destructive">{t('projects.badge.missingDir')}</Badge>
										{/if}
										{#if project.duplicateGroup}
											<Badge variant="secondary">{t('projects.badge.duplicate')}</Badge>
										{/if}
									</div>
								</Table.Cell>
								<Table.Cell class="text-right tabular-nums">{exact(project.sessions)}</Table.Cell>
								<Table.Cell class="hidden text-right tabular-nums lg:table-cell"
									>{exact(project.turns)}</Table.Cell
								>
								<Table.Cell class="hidden text-right tabular-nums xl:table-cell">
									{compact(project.inputTokens)}
								</Table.Cell>
								<Table.Cell class="hidden text-right tabular-nums xl:table-cell">
									{compact(project.outputTokens)}
								</Table.Cell>
								<Table.Cell class="hidden text-right tabular-nums md:table-cell">
									{#if project.transcriptFiles > 0}
										{exact(project.transcriptFiles)} · {formatBytes(project.transcriptBytes)}
									{:else}
										—
									{/if}
								</Table.Cell>
								<Table.Cell class="hidden whitespace-nowrap md:table-cell"
									>{formatWhen(project.lastTs)}</Table.Cell
								>
								<Table.Cell>
									<div class="flex justify-end gap-1">
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
	<AlertDialog.Content class="max-w-xl">
		<AlertDialog.Header>
			<AlertDialog.Title>{t('projects.delete.title')}</AlertDialog.Title>
			<AlertDialog.Description>
				{t('projects.delete.body', { path: deleteTarget?.transcriptDir ?? '' })}
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

				{#if table.rows.length === 0}
					<p class="p-4 text-sm text-muted-foreground">{t('common.noMatch')}</p>
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
			<Button
				variant="destructive"
				disabled={deleteBusy || !deleteFiles || deleteFiles.length === 0}
				onclick={confirmDelete}
			>
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
			<AlertDialog.Description>
				{t('projects.remove.body', { path: removeTarget?.path ?? '' })}
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
	<Dialog.Content class="max-w-2xl">
		<Dialog.Header>
			<Dialog.Title>{t('projects.settings.title')}</Dialog.Title>
			<Dialog.Description>
				{t('projects.settings.body', { path: settingsTarget?.path ?? '' })}
			</Dialog.Description>
		</Dialog.Header>

		<Textarea bind:value={settingsText} class="min-h-72 font-mono text-xs" spellcheck={false} />

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
