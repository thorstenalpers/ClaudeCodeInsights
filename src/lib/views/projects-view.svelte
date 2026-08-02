<script lang="ts">
	import Pencil from '@lucide/svelte/icons/pencil';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Unlink from '@lucide/svelte/icons/unlink';
	import { api, type ProjectRow, type ProjectsReport, type TranscriptFile } from '$lib/api';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
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

	const deleteTotalBytes = $derived(
		deleteFiles?.reduce((sum, file) => sum + file.sizeBytes, 0) ?? 0
	);
</script>

<div class="flex h-full flex-col gap-4 p-6">
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
				class="min-h-0 flex-1 overflow-auto rounded-md border [&>[data-slot=table-container]]:overflow-visible"
			>
				<Table.Root>
					<Table.Header class="sticky top-0 z-10 bg-background">
						<Table.Row>
							<Table.Head>{t('projects.column.project')}</Table.Head>
							<Table.Head class="w-40">{t('projects.column.status')}</Table.Head>
							<Table.Head class="text-right">{t('projects.column.sessions')}</Table.Head>
							<Table.Head class="text-right">{t('projects.column.turns')}</Table.Head>
							<Table.Head class="text-right">{t('projects.column.input')}</Table.Head>
							<Table.Head class="text-right">{t('projects.column.output')}</Table.Head>
							<Table.Head class="text-right">{t('projects.column.transcripts')}</Table.Head>
							<Table.Head>{t('projects.column.lastActive')}</Table.Head>
							<Table.Head class="w-28"></Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each report.projects as project (project.path + (project.transcriptDir ?? ''))}
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
								<Table.Cell class="text-right tabular-nums">{exact(project.turns)}</Table.Cell>
								<Table.Cell class="text-right tabular-nums">
									{compact(project.inputTokens)}
								</Table.Cell>
								<Table.Cell class="text-right tabular-nums">
									{compact(project.outputTokens)}
								</Table.Cell>
								<Table.Cell class="text-right tabular-nums">
									{#if project.transcriptFiles > 0}
										{exact(project.transcriptFiles)} · {formatBytes(project.transcriptBytes)}
									{:else}
										—
									{/if}
								</Table.Cell>
								<Table.Cell class="whitespace-nowrap">{formatWhen(project.lastTs)}</Table.Cell>
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
				class="max-h-64 overflow-auto rounded-md border [&>[data-slot=table-container]]:overflow-visible"
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
