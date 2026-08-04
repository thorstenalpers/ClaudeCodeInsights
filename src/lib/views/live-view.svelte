<script lang="ts">
	/**
	 * What Claude Code is doing right now.
	 *
	 * The host follows the transcript being written and pushes each new turn;
	 * this page only appends. Nothing is polled, so an idle session costs
	 * nothing and a busy one arrives as fast as it is written.
	 */
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import CircleDot from '@lucide/svelte/icons/circle-dot';
	import GitBranch from '@lucide/svelte/icons/git-branch';
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
	import { displayPath, formatSecond } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';

	/** One session in the tree, with what it takes to tell it from its neighbour. */
	type Session = {
		id: string;
		project: string | null;
		at: string | null;
		branch: string | null;
		model: string | null;
		/** How many lines of it are in the buffer, which is how busy it looks. */
		lines: number;
	};

	/** Enough to see what led here, not so much that the page is a transcript. */
	const TAIL = 20;
	/** Old turns are dropped rather than kept: this is a window, not an archive. */
	const KEEP = 200;
	const PINS_KEY = 'claudeadmin.live.pins';

	let turns = $state<LiveTurn[]>([]);
	let error = $state<string | null>(null);
	let following = $state(false);
	let lastAt = $state<number | null>(null);
	/** Which session is open in the tree; empty means the most recent one. */
	let openTab = $state<string>('');
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
		const seen: Record<string, Session> = {};
		for (const turn of turns) {
			const known = seen[turn.sessionId];
			if (known) {
				known.project = turn.project ?? known.project;
				known.at = turn.timestamp ?? known.at;
				known.branch = turn.gitBranch ?? known.branch;
				known.model = turn.model ?? known.model;
				known.lines += 1;
			} else {
				seen[turn.sessionId] = {
					id: turn.sessionId,
					project: turn.project ?? null,
					at: turn.timestamp ?? null,
					branch: turn.gitBranch ?? null,
					model: turn.model ?? null,
					lines: 1
				};
			}
		}
		return Object.values(seen).sort((a, b) => (b.at ?? '').localeCompare(a.at ?? ''));
	});

	const visible = $derived(
		// Pinned first, and among equals the most recent: a pin is a promise that
		// the session stays where it was put.
		[...tabs].sort((a, b) => Number(pinned.includes(b.id)) - Number(pinned.includes(a.id)))
	);

	/**
	 * The tree on the left: a project, and the sessions running in it.
	 *
	 * The folder name alone does not place a session — two checkouts of the same
	 * repository read alike, and one project often has several sessions at once.
	 * So a session brings its id, its branch, its model and when it last said
	 * something, and the project brings the count.
	 */
	const projects = $derived.by(() => {
		const seen: Record<string, { path: string; sessions: Session[] }> = {};
		for (const session of visible) {
			const path = session.project ?? '';
			seen[path] ??= { path, sessions: [] };
			seen[path].sessions.push(session);
		}
		return Object.values(seen);
	});

	/** Projects the reader has folded away; everything starts open. */
	let folded = $state<string[]>([]);

	function toggleFold(path: string): void {
		folded = folded.includes(path) ? folded.filter((entry) => entry !== path) : [...folded, path];
	}

	const current = $derived(visible.find((tab) => tab.id === openTab) ?? visible[0]);
	const shown = $derived(turns.filter((turn) => turn.sessionId === current?.id));

	/**
	 * The table is what the page opens with.
	 *
	 * A live view is read for what is happening — which tool, how long ago, at
	 * what cost — and the table answers that at a glance. The stream is the
	 * other question, what was actually said, and is one button away.
	 */
	let asTable = $state(true);

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
					{t('live.lastAt', { time: formatSecond(new Date(lastAt).toISOString()) })}
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

		<div class="flex min-h-0 flex-1 gap-3">
			<!-- The sessions, under the projects they run in. A folder name is not
			     an address: two checkouts read alike and one project often has
			     several sessions at once, so each carries its id, its branch and
			     when it last said something. -->
			<aside
				class="hidden w-64 shrink-0 flex-col overflow-auto rounded-md border p-1 @2xl:flex"
				aria-label={t('live.sessions')}
			>
				{#each projects as project (project.path)}
					{@const open = !folded.includes(project.path)}
					<button
						type="button"
						class="flex w-full items-center gap-1 rounded-md px-1.5 py-1 text-left hover:bg-accent"
						onclick={() => toggleFold(project.path)}
					>
						<ChevronRight
							class={['size-3 shrink-0 transition-transform duration-150', open && 'rotate-90']}
						/>
						<span class="min-w-0 flex-1 truncate text-xs font-medium" title={project.path}>
							{project.path ? displayPath(project.path) : t('live.noProject')}
						</span>
						<span class="text-[11px] text-muted-foreground tabular-nums">
							{project.sessions.length}
						</span>
					</button>

					{#if open}
						{#each project.sessions as session (session.id)}
							<button
								type="button"
								class={[
									'ml-3 flex flex-col gap-0.5 rounded-md border-l px-2 py-1 text-left',
									current?.id === session.id ? 'border-l-primary bg-primary/10' : 'hover:bg-accent'
								]}
								onclick={() => (openTab = session.id)}
							>
								<span class="flex items-center gap-1">
									{#if pinned.includes(session.id)}
										<Pin class="size-3 shrink-0 text-muted-foreground" />
									{/if}
									<span class="font-mono text-xs" title={session.id}>
										{session.id.slice(0, 8)}
									</span>
									<span class="ml-auto text-[11px] text-muted-foreground tabular-nums">
										{formatSecond(session.at)}
									</span>
								</span>
								<span class="flex items-center gap-1 text-[11px] text-muted-foreground">
									{#if session.branch}
										<GitBranch class="size-3 shrink-0" />
										<span class="min-w-0 truncate">{session.branch}</span>
									{/if}
									<span class="ml-auto shrink-0 tabular-nums">
										{t('live.lines', { count: String(session.lines) })}
									</span>
								</span>
							</button>
						{/each}
					{/if}
				{/each}

				{#if projects.length === 0}
					<p class="p-2 text-xs text-muted-foreground">{t('live.waiting')}</p>
				{/if}
			</aside>

			<div class="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
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
											{formatSecond(turn.timestamp)}
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
											<span class="tabular-nums">{formatSecond(turn.timestamp)}</span>
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
													<Badge
														variant="outline"
														class="h-5 px-1.5 font-mono text-[11px] font-normal"
													>
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
			</div>
		</div>

		<p class="shrink-0 text-xs text-muted-foreground">{t('live.readOnly')}</p>
	{/if}
</div>
