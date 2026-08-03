<script lang="ts">
	/**
	 * What Claude Code is doing right now.
	 *
	 * The host follows the transcript being written and pushes each new turn;
	 * this page only appends. Nothing is polled, so an idle session costs
	 * nothing and a busy one arrives as fast as it is written.
	 */
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import CircleDot from '@lucide/svelte/icons/circle-dot';
	import Pin from '@lucide/svelte/icons/pin';
	import PinOff from '@lucide/svelte/icons/pin-off';
	import RotateCw from '@lucide/svelte/icons/rotate-cw';
	import TableIcon from '@lucide/svelte/icons/table';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import { Button } from '$lib/components/ui/button';
	import { api, type LiveTurn } from '$lib/api';
	import ResetView from '$lib/components/reset-view.svelte';
	import SortHeader from '$lib/components/sort-header.svelte';
	import { Input } from '$lib/components/ui/input';
	import * as Table from '$lib/components/ui/table';
	import { compact } from '$lib/format';
	import { costOf } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { createTable } from '$lib/table.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import * as Card from '$lib/components/ui/card';
	import { displayPath, formatTime } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';

	/** Enough to see what led here, not so much that the page is a transcript. */
	const TAIL = 20;
	/** Old turns are dropped rather than kept: this is a window, not an archive. */
	const KEEP = 200;
	const PINS_KEY = 'claudeadmin.live.pins';

	let turns = $state<LiveTurn[]>([]);
	let error = $state<string | null>(null);
	let following = $state(false);
	let lastAt = $state<number | null>(null);
	/** Which session's tab is open; empty means the busiest one. */
	let openTab = $state<string>('');
	/** Which project the strip is narrowed to; empty means all of them. */
	let openProject = $state<string>('');
	let reloading = $state(false);
	/** Bumped to make the effect run again, which is what reload means here. */
	let reloadKey = $state(0);

	/**
	 * Sessions the reader wants to keep in front.
	 *
	 * Kept across restarts: a session worth pinning is one being watched over a
	 * longer stretch than a window stays open.
	 */
	let pinned = $state<string[]>(readPins());

	function readPins(): string[] {
		if (typeof localStorage === 'undefined') return [];
		try {
			const stored: unknown = JSON.parse(localStorage.getItem(PINS_KEY) ?? '[]');
			return Array.isArray(stored) ? stored.filter((id) => typeof id === 'string') : [];
		} catch {
			return [];
		}
	}

	function togglePin(id: string): void {
		pinned = pinned.includes(id) ? pinned.filter((entry) => entry !== id) : [...pinned, id];
		localStorage.setItem(PINS_KEY, JSON.stringify(pinned));
	}

	/**
	 * One tab per session, busiest first.
	 *
	 * Several sessions run side by side, and merging them into one stream reads
	 * as one confused conversation. The order is by last activity so the tab a
	 * reader wants is the one already in front.
	 */
	const tabs = $derived.by(() => {
		// A plain object, not a Map: this is rebuilt from scratch on every run
		// and never observed, which is what the reactive Map is for.
		const seen: Record<string, { id: string; project: string | null; at: string | null }> = {};
		for (const turn of turns) {
			const known = seen[turn.sessionId];
			if (known) {
				known.project = turn.project ?? known.project;
				known.at = turn.timestamp ?? known.at;
			} else {
				seen[turn.sessionId] = {
					id: turn.sessionId,
					project: turn.project ?? null,
					at: turn.timestamp ?? null
				};
			}
		}
		return Object.values(seen).sort((a, b) => (b.at ?? '').localeCompare(a.at ?? ''));
	});

	/**
	 * The projects behind those sessions, busiest first.
	 *
	 * A machine running several checkouts at once produces a strip of tabs that
	 * all read alike; the project is the coarse cut a reader makes first.
	 */
	const projects = $derived.by(() => {
		const seen: Record<string, { path: string; sessions: number }> = {};
		for (const tab of tabs) {
			const path = tab.project ?? '';
			seen[path] ??= { path, sessions: 0 };
			seen[path].sessions += 1;
		}
		return Object.values(seen);
	});

	const visible = $derived(
		tabs
			.filter((tab) => openProject === '' || (tab.project ?? '') === openProject)
			// Pinned first, and among equals the most recent: a pin is a promise
			// that the tab stays where it was put.
			.sort((a, b) => Number(pinned.includes(b.id)) - Number(pinned.includes(a.id)))
	);

	// A project whose last session has aged out of the buffer would otherwise
	// leave the page filtered to nothing, with the button to undo it gone too.
	$effect(() => {
		if (openProject !== '' && visible.length === 0 && tabs.length > 0) openProject = '';
	});

	const current = $derived(visible.find((tab) => tab.id === openTab) ?? visible[0]);
	const shown = $derived(turns.filter((turn) => turn.sessionId === current?.id));

	/** The stream reads as a conversation; the table answers questions about it. */
	let asTable = $state(false);

	const cost = (turn: LiveTurn) => (turn.model ? costOf(turn.model, turn) : 0);
	const tokens = (turn: LiveTurn) =>
		turn.inputTokens + turn.outputTokens + turn.cacheReadTokens + turn.cacheWriteTokens;

	const COLUMNS = [
		{ id: 'time', label: 'logs.column.time' as const },
		{ id: 'role', label: 'live.column.role' as const },
		{ id: 'kind', label: 'live.column.kind' as const },
		{
			id: 'model',
			label: 'sessions.column.model' as const,
			class: 'hidden @xl:table-cell'
		},
		{ id: 'tools', label: 'nav.tools' as const, class: 'hidden @lg:table-cell' },
		{ id: 'tokens', label: 'agents.column.tokens' as const, numeric: true },
		{ id: 'cost', label: 'sessions.column.cost' as const, numeric: true },
		{ id: 'text', label: 'live.column.text' as const }
	];

	const CLASS: Record<string, string> = Object.fromEntries(
		COLUMNS.map((column) => [column.id, 'class' in column ? (column.class ?? '') : ''])
	);

	const table = createTable<LiveTurn>(
		() => shown,
		{
			time: (turn) => turn.timestamp ?? '',
			role: (turn) => turn.role,
			// The thinking flag rides in the kind column: it is the same question
			// — what was this line doing — and it costs no width.
			kind: (turn) => (turn.thinking ? `${turn.kind}+` : turn.kind),
			model: (turn) => shortModel(turn.model ?? ''),
			tools: (turn) => turn.tools.join(', '),
			tokens,
			cost,
			text: (turn) => turn.text ?? ''
		},
		{ sort: 'time', descending: false }
	);

	function shortModel(model: string): string {
		return model.replace(/^claude-/, '');
	}

	/** Re-reads every followed transcript from the start. */
	async function reload(): Promise<void> {
		if (reloading) return;
		reloading = true;
		await api.stopLive().catch(() => {});
		reloadKey += 1;
		reloading = false;
	}

	$effect(() => {
		void reloadKey;
		if (!isHosted) return;

		let stop: UnlistenFn | undefined;
		let cancelled = false;

		void (async () => {
			try {
				const recent = await api.startLive(TAIL);
				if (cancelled) return;
				turns = recent;
				following = true;
				lastAt = recent.length > 0 ? Date.now() : null;
			} catch (cause) {
				if (!cancelled) error = errorMessage(cause);
				return;
			}

			stop = await listen<LiveTurn>('live:turn', (event) => {
				turns = [...turns, event.payload].slice(-KEEP);
				lastAt = Date.now();
			});
		})();

		return () => {
			cancelled = true;
			stop?.();
			void api.stopLive();
			following = false;
		};
	});
</script>

<div class="flex h-full flex-col gap-3 p-4">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('live.idleTitle')}</Card.Title>
				<Card.Description>{t('live.idleBody')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else}
		<div class="flex shrink-0 flex-wrap items-center gap-2">
			<Badge variant={following ? 'default' : 'outline'} class="gap-1 font-normal">
				<CircleDot class="size-3" />
				{following ? t('live.following') : t('live.stopped')}
			</Badge>
			{#if current}
				<!-- The id in the title, because a project name does not place a
				     session and two of them can be running side by side. -->
				<span class="flex min-w-0 items-center gap-1.5">
					{#if pinned.includes(current.id)}
						<Pin class="size-3 shrink-0 text-muted-foreground" />
					{/if}
					<span class="font-mono text-xs" title={current.id}>{current.id.slice(0, 8)}</span>
					{#if current.project}
						<span class="truncate font-mono text-xs text-muted-foreground">
							· {displayPath(current.project)}
						</span>
					{/if}
				</span>
			{/if}
			{#if lastAt}
				<span class="ml-auto text-xs text-muted-foreground">
					{t('live.lastAt', { time: formatTime(new Date(lastAt).toISOString()) })}
				</span>
			{/if}
			{#if current}
				{@const isPinned = pinned.includes(current.id)}
				<Button
					variant={isPinned ? 'secondary' : 'outline'}
					size="sm"
					class={lastAt ? '' : 'ml-auto'}
					onclick={() => togglePin(current.id)}
				>
					{#if isPinned}
						<PinOff />
					{:else}
						<Pin />
					{/if}
					{t(isPinned ? 'live.unpin' : 'live.pin')}
				</Button>
			{/if}

			<Button variant="outline" size="sm" onclick={() => (asTable = !asTable)}>
				{#if asTable}
					<MessageSquare />
				{:else}
					<TableIcon />
				{/if}
				{t(asTable ? 'live.asStream' : 'live.asTable')}
			</Button>

			<Button variant="outline" size="sm" disabled={reloading} onclick={() => void reload()}>
				<RotateCw class={reloading ? 'animate-spin' : ''} />
				{t('live.reload')}
			</Button>
		</div>

		{#if projects.length > 1}
			<!-- The coarse cut, above the sessions: only worth showing when more
			     than one project is running. -->
			<div class="flex shrink-0 gap-1 overflow-x-auto">
				<Button
					variant={openProject === '' ? 'secondary' : 'ghost'}
					size="sm"
					class="shrink-0 font-normal"
					onclick={() => (openProject = '')}
				>
					{t('live.allProjects', { count: String(tabs.length) })}
				</Button>
				{#each projects as project (project.path)}
					<Button
						variant={openProject === project.path ? 'secondary' : 'ghost'}
						size="sm"
						class="shrink-0 font-normal"
						onclick={() => (openProject = project.path)}
					>
						<span class="max-w-48 truncate">
							{project.path ? displayPath(project.path) : t('live.noProject')}
						</span>
						<span class="text-xs text-muted-foreground tabular-nums">{project.sessions}</span>
					</Button>
				{/each}
			</div>
		{/if}

		{#if visible.length > 1}
			<!-- Only when there is a choice: one session needs no tab strip. -->
			<div class="flex shrink-0 gap-1 overflow-x-auto border-b pb-1">
				{#each visible as tab (tab.id)}
					<Button
						variant={current?.id === tab.id ? 'secondary' : 'ghost'}
						size="sm"
						class="shrink-0 font-normal"
						onclick={() => (openTab = tab.id)}
					>
						{#if pinned.includes(tab.id)}
							<Pin class="size-3 shrink-0" />
						{/if}
						<span class="max-w-40 truncate">
							{tab.project ? displayPath(tab.project) : tab.id.slice(0, 8)}
						</span>
					</Button>
				{/each}
			</div>
		{/if}

		{#if asTable}
			<div class="flex shrink-0 flex-wrap items-center gap-2">
				<Input placeholder={t('common.search')} class="h-8 max-w-xs" bind:value={table.query} />
				<ResetView show={table.dirty} onreset={() => table.reset()} />
				<span class="ml-auto text-xs text-muted-foreground tabular-nums">
					{table.rows.length}
				</span>
			</div>

			<div
				class="min-h-0 flex-1 overflow-auto rounded-md border [&_td]:py-1 [&_td]:text-xs [&_th]:h-8 [&>[data-slot=table-container]]:overflow-visible"
			>
				<Table.Root>
					<Table.Header class="sticky top-0 z-10 bg-background">
						<Table.Row>
							{#each COLUMNS as column (column.id)}
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
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each table.rows as turn, index (index)}
							<Table.Row>
								<Table.Cell class="whitespace-nowrap text-muted-foreground/60 tabular-nums">
									{formatTime(turn.timestamp)}
								</Table.Cell>
								<Table.Cell class="whitespace-nowrap">
									{turn.role === 'user' ? t('transcript.you') : t('transcript.claude')}
									{#if turn.agent}
										<span class="text-muted-foreground">·{t('live.column.agent')}</span>
									{/if}
								</Table.Cell>
								<Table.Cell class="whitespace-nowrap">
									<Badge variant="outline" class="h-5 px-1.5 font-normal">
										{t(turn.kind === 'code' ? 'live.kind.code' : 'live.kind.chat')}
									</Badge>
									{#if turn.thinking}
										<Badge variant="outline" class="h-5 px-1.5 font-normal">
											{t('live.kind.thinking')}
										</Badge>
									{/if}
								</Table.Cell>
								<Table.Cell class={[CLASS.model, 'whitespace-nowrap text-muted-foreground']}>
									{shortModel(turn.model ?? '')}
								</Table.Cell>
								<Table.Cell class={[CLASS.tools, 'max-w-40 truncate font-mono']}>
									{turn.tools.join(', ')}
								</Table.Cell>
								<Table.Cell class="text-right tabular-nums">
									{tokens(turn) === 0 ? '' : compact(tokens(turn))}
								</Table.Cell>
								<Table.Cell class="text-right tabular-nums">
									{cost(turn) === 0 ? '' : region.format(cost(turn))}
								</Table.Cell>
								<Table.Cell class="max-w-96 truncate" title={turn.text ?? ''}>
									{turn.text ?? ''}
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>

				{#if table.rows.length === 0}
					<p class="p-4 text-sm text-muted-foreground">{t('live.waiting')}</p>
				{/if}
			</div>
		{:else}
			<div class="min-h-0 flex-1 overflow-auto rounded-md border p-3">
				{#if shown.length === 0}
					<p class="text-sm text-muted-foreground">{t('live.waiting')}</p>
				{:else}
					<div class="flex flex-col gap-3">
						{#each shown as turn, index (index)}
							<div class="flex flex-col gap-1">
								<div class="flex items-center gap-2 text-xs text-muted-foreground">
									<span class={['font-medium', turn.role === 'user' && 'text-foreground']}>
										{turn.role === 'user' ? t('transcript.you') : t('transcript.claude')}
									</span>
									<span class="tabular-nums">{formatTime(turn.timestamp)}</span>
								</div>

								{#if turn.text}
									<div
										class={[
											'text-sm leading-relaxed whitespace-pre-wrap',
											turn.role === 'user' && 'rounded-lg bg-muted px-3 py-2'
										]}
									>
										{turn.text}
									</div>
								{/if}

								{#if turn.tools.length > 0}
									<div class="flex flex-wrap gap-1">
										{#each turn.tools as tool, position (position)}
											<Badge variant="outline" class="h-5 px-1.5 font-mono text-[11px] font-normal">
												{tool}
											</Badge>
										{/each}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}

		<p class="shrink-0 text-xs text-muted-foreground">{t('live.readOnly')}</p>
	{/if}
</div>
