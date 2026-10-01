<script lang="ts">
	import Brain from '@lucide/svelte/icons/brain';
	import CircleDot from '@lucide/svelte/icons/circle-dot';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import {
		api,
		type LiveTurn,
		type SessionRow,
		type TranscriptPage,
		type TranscriptTurn
	} from '$lib/api';
	import { Badge } from '$lib/components/ui/badge';
	import VitalsStrip, { type Vital } from '$lib/components/vitals-strip.svelte';
	import ToolCallCard from '$lib/components/tool-call-card.svelte';
	import ResetView from '$lib/components/reset-view.svelte';
	import SortHeader from '$lib/components/sort-header.svelte';
	import { Input } from '$lib/components/ui/input';
	import * as Table from '$lib/components/ui/table';
	import { createTable } from '$lib/table.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { compact, exact, formatDuration, formatTime, sourceName } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { costOf } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';

	type Props = { sessionId: string };
	let { sessionId }: Props = $props();

	const PAGE = 100;

	let page = $state<TranscriptPage | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let showThinking = $state(false);
	let showTools = $state(true);
	let expandedThinking = $state<Record<number, boolean>>({});

	$effect(() => {
		const id = sessionId;
		if (!isHosted) {
			loading = false;
			return;
		}

		let cancelled = false;
		loading = true;
		error = null;
		page = null;

		api
			.getTranscript(id, 0, PAGE)
			.then((value) => {
				if (!cancelled) page = value;
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

	/**
	 * The session as it is still being written.
	 *
	 * The transcript above comes from the database, which knows what the last
	 * scan saw. The host follows the files that are being appended to right now
	 * and pushes each new line; the ones belonging to this session are added
	 * here, so an open session reads on rather than stopping at the last scan.
	 */
	let live = $state<TranscriptTurn[]>([]);
	let following = $state(false);

	$effect(() => {
		const id = sessionId;
		if (!isHosted) return;

		let stop: UnlistenFn | undefined;
		let cancelled = false;
		live = [];

		void (async () => {
			// The tail is already on screen from the transcript; only what comes
			// after starting to watch is new.
			await api.startLive(0).catch(() => []);
			if (cancelled) return;
			following = true;

			stop = await listen<LiveTurn>('live:turn', (event) => {
				if (event.payload.sessionId !== id) return;
				live = [...live, asTurn(event.payload, live.length)];
			});
		})();

		return () => {
			cancelled = true;
			stop?.();
			void api.stopLive();
			following = false;
		};
	});

	/** A live line in the shape the transcript below already draws. */
	function asTurn(line: LiveTurn, index: number): TranscriptTurn {
		return {
			index: -1 - index,
			role: line.role === 'user' ? 'user' : 'assistant',
			timestamp: line.timestamp,
			text: line.text,
			thinking: null,
			toolCalls: line.tools.map((name) => ({
				name,
				input: '',
				inputTruncated: false,
				result: null,
				resultTruncated: false,
				isError: false
			}))
		};
	}

	async function loadMore() {
		if (!page) return;
		const next = await api.getTranscript(sessionId, page.turns.length, PAGE);
		page = { ...next, turns: [...page.turns, ...next.turns], offset: 0 };
	}

	// The condition must match what the markup actually renders. Testing
	// `turn.thinking !== null` regardless of the toggle let thinking-only turns
	// through as a bare timestamp with nothing under it.
	const visible = $derived(
		page
			? [...page.turns, ...live].filter(
					(turn) =>
						turn.text !== null ||
						(showThinking && turn.thinking !== null) ||
						(showTools && turn.toolCalls.length > 0)
				)
			: []
	);

	/** The transcript reads as a conversation; the table answers about its parts. */
	let asTable = $state(false);

	const COLUMNS = [
		{ id: 'index', label: 'transcript.column.turn' as const, numeric: true },
		{ id: 'time', label: 'logs.column.time' as const },
		{ id: 'role', label: 'live.column.role' as const },
		{ id: 'kind', label: 'live.column.kind' as const },
		{ id: 'tools', label: 'nav.tools' as const, class: 'hidden @lg:table-cell' },
		{ id: 'text', label: 'live.column.text' as const }
	];

	const CLASS: Record<string, string> = Object.fromEntries(
		COLUMNS.map((column) => [column.id, 'class' in column ? (column.class ?? '') : ''])
	);

	/** What a turn was: it can be several at once, so they are joined. */
	function kindOf(turn: TranscriptTurn): string {
		const parts: string[] = [];
		if (turn.text) parts.push(t('live.kind.chat'));
		if (turn.thinking) parts.push(t('live.kind.thinking'));
		if (turn.toolCalls.length > 0) parts.push(t('live.kind.code'));
		return parts.join(' · ');
	}

	const table = createTable<TranscriptTurn>(
		() => visible,
		{
			index: (turn) => turn.index,
			time: (turn) => turn.timestamp ?? '',
			role: (turn) => turn.role,
			kind: kindOf,
			tools: (turn) => turn.toolCalls.map((call) => call.name).join(', '),
			text: (turn) => turn.text ?? turn.thinking ?? ''
		},
		{ sort: 'index', descending: false }
	);

	// The figures come from the database while the transcript above comes from
	// the file, so a session written since the last scan shows the conversation
	// with no strip rather than a strip of zeroes.
	let row = $state<SessionRow | null>(null);

	$effect(() => {
		const id = sessionId;
		if (!isHosted) return;

		let cancelled = false;
		row = null;
		api
			.getSession(id)
			.then((value) => {
				if (!cancelled) row = value;
			})
			.catch(() => {});

		return () => {
			cancelled = true;
		};
	});

	const vitals = $derived.by((): Vital[] => {
		if (!row) return [];
		const tokens = row.inputTokens + row.outputTokens + row.cacheReadTokens + row.cacheWriteTokens;
		// A row with no model cannot be priced — a dash says unknown, where a
		// zero would say free.
		const cost = row.model
			? costOf(row.model, {
					inputTokens: row.inputTokens,
					outputTokens: row.outputTokens,
					cacheReadTokens: row.cacheReadTokens,
					cacheWriteTokens: row.cacheWriteTokens
				})
			: null;

		const cells: Vital[] = [
			{
				label: t('sessions.column.duration'),
				value: formatDuration(row.durationMinutes),
				sublabel: `${formatTime(row.firstTs)} – ${formatTime(row.lastTs)}`,
				info: t('info.duration')
			},
			{
				label: t('sessions.column.turns'),
				value: exact(row.turnCount),
				sublabel: row.hasSubagents ? t('sessions.subagents') : undefined,
				info: t('info.turns')
			},
			{
				label: t('cost.summary.tokens'),
				value: compact(tokens),
				sublabel: t('cost.summary.tokens.hint', {
					input: compact(row.inputTokens),
					output: compact(row.outputTokens),
					cache: compact(row.cacheReadTokens + row.cacheWriteTokens)
				}),
				info: t('info.tokens')
			},
			{
				label: t('sessions.column.cost'),
				value: cost === null ? t('common.none') : region.format(cost),
				// Named only when it is not the default agent: on those rows the
				// model name alone would leave the reader guessing who ran it.
				sublabel:
					row.source !== 'claude'
						? `${row.model ?? '—'} · ${sourceName(row.source)}`
						: (row.model ?? undefined),
				info: t('info.cost')
			},
			{
				label: t('sessions.column.files'),
				value: exact(row.filesTouched),
				info: t('info.files')
			}
		];

		// Only shown when it happened: a zero here would be one more cell to read
		// past on every session that never came close to the limit.
		const compactions = row.compactAuto + row.compactManual;
		if (compactions > 0) {
			cells.push({
				label: t('sessions.column.compacts'),
				value: exact(compactions),
				sublabel: t('sessions.compactSplit', {
					auto: String(row.compactAuto),
					manual: String(row.compactManual)
				}),
				info: t('info.compacts'),
				warn: row.compactAuto > 0
			});
		}

		return cells;
	});
</script>

<div class="flex h-full flex-col">
	{#if vitals.length > 0}
		<div class="shrink-0 px-6 pt-3">
			<VitalsStrip {vitals} />
		</div>
	{/if}

	<div class="flex shrink-0 flex-wrap items-center gap-2 border-b px-6 py-3">
		<Button
			variant={showTools ? 'default' : 'outline'}
			size="sm"
			class="h-7 px-2 text-xs font-normal"
			onclick={() => (showTools = !showTools)}
		>
			{t('transcript.tools')}
		</Button>
		<Button
			variant={showThinking ? 'default' : 'outline'}
			size="sm"
			class="h-7 px-2 text-xs font-normal"
			onclick={() => (showThinking = !showThinking)}
		>
			{t('transcript.thinking')}
		</Button>

		<Button
			variant={asTable ? 'default' : 'outline'}
			size="sm"
			class="h-7 px-2 text-xs font-normal"
			onclick={() => (asTable = !asTable)}
		>
			{t(asTable ? 'live.asStream' : 'live.asTable')}
		</Button>

		{#if asTable}
			<Input placeholder={t('common.search')} class="h-7 max-w-56" bind:value={table.query} />
			<ResetView show={table.dirty} onreset={() => table.reset()} />
		{/if}

		{#if following}
			<!-- Only while lines are actually arriving: a badge that is always on
			     says nothing about whether this session is still being written. -->
			<Badge variant={live.length > 0 ? 'default' : 'outline'} class="gap-1 font-normal">
				<CircleDot class="size-3" />
				{live.length > 0 ? t('live.following') : t('live.stopped')}
			</Badge>
		{/if}

		{#if page}
			<span class="ml-auto text-xs text-muted-foreground tabular-nums">
				{t('transcript.turnCount', {
					shown: page.turns.length + live.length,
					total: page.total + live.length
				})}
			</span>
		{/if}
	</div>

	<div class="min-h-0 flex-1 overflow-auto">
		<div class="mx-auto flex max-w-3xl flex-col gap-4 p-6">
			{#if !isHosted}
				<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
			{:else if loading}
				{#each [...Array(5).keys()] as index (index)}
					<Skeleton class="h-20 w-full" />
				{/each}
			{:else if error}
				<Card.Root>
					<Card.Header>
						<Card.Title>{t('transcript.loadFailed')}</Card.Title>
						<Card.Description class="font-mono text-xs">{error}</Card.Description>
					</Card.Header>
				</Card.Root>
			{:else if page && page.path === null}
				<Card.Root>
					<Card.Header>
						<Card.Title>{t('transcript.missingTitle')}</Card.Title>
						<Card.Description>{t('transcript.missingBody')}</Card.Description>
					</Card.Header>
				</Card.Root>
			{:else if visible.length === 0}
				<p class="text-sm text-muted-foreground">{t('transcript.filteredOut')}</p>
			{:else if asTable}
				<div
					class="overflow-auto rounded-md border [&_td]:py-1 [&_td]:text-xs [&_th]:h-8 [&>[data-slot=table-container]]:overflow-visible"
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
							{#each table.rows as turn (turn.index)}
								<Table.Row>
									<Table.Cell class="text-right tabular-nums">{turn.index + 1}</Table.Cell>
									<Table.Cell class="whitespace-nowrap text-muted-foreground/60 tabular-nums">
										{formatTime(turn.timestamp)}
									</Table.Cell>
									<Table.Cell class="whitespace-nowrap">
										{turn.role === 'user' ? t('transcript.you') : t('transcript.claude')}
									</Table.Cell>
									<Table.Cell class="whitespace-nowrap text-muted-foreground">
										{kindOf(turn)}
									</Table.Cell>
									<Table.Cell class={[CLASS.tools, 'max-w-40 truncate font-mono']}>
										{turn.toolCalls.map((call) => call.name).join(', ')}
									</Table.Cell>
									<Table.Cell class="max-w-96 truncate" title={turn.text ?? turn.thinking ?? ''}>
										{turn.text ?? turn.thinking ?? ''}
									</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				</div>
			{:else}
				{#each visible as turn (turn.index)}
					<div class="flex flex-col gap-2">
						<div class="flex items-center gap-2 text-xs text-muted-foreground">
							<span class="font-medium {turn.role === 'user' ? 'text-foreground' : ''}">
								{turn.role === 'user' ? t('transcript.you') : t('transcript.claude')}
							</span>
							<span class="tabular-nums">{formatTime(turn.timestamp)}</span>
						</div>

						{#if turn.thinking && showThinking}
							<div class="rounded-md border border-dashed bg-muted/50">
								<button
									type="button"
									class="flex w-full items-center gap-2 px-3 py-2 text-left text-xs text-muted-foreground"
									onclick={() =>
										(expandedThinking = {
											...expandedThinking,
											[turn.index]: !expandedThinking[turn.index]
										})}
								>
									<Brain class="size-3.5" />
									{t('transcript.thinking')}
								</button>
								{#if expandedThinking[turn.index]}
									<p
										class="border-t px-3 py-2 text-sm leading-relaxed whitespace-pre-wrap text-muted-foreground"
									>
										{turn.thinking}
									</p>
								{/if}
							</div>
						{/if}

						{#if turn.text}
							<div
								class={turn.role === 'user'
									? 'rounded-lg bg-muted px-3 py-2 text-sm leading-relaxed whitespace-pre-wrap'
									: 'text-sm leading-relaxed whitespace-pre-wrap'}
							>
								{turn.text}
							</div>
						{/if}

						{#if showTools && turn.toolCalls.length > 0}
							<div class="flex flex-col gap-1.5">
								{#each turn.toolCalls as call, callIndex (callIndex)}
									<ToolCallCard {call} />
								{/each}
							</div>
						{/if}
					</div>
				{/each}

				{#if page && page.turns.length < page.total}
					<Button variant="outline" size="sm" class="self-center" onclick={loadMore}>
						{t('transcript.loadMore', { count: Math.min(PAGE, page.total - page.turns.length) })}
					</Button>
				{/if}
			{/if}
		</div>
	</div>
</div>
