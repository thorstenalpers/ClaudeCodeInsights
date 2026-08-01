<script lang="ts">
	import Brain from '@lucide/svelte/icons/brain';
	import { api, type TranscriptPage } from '$lib/api';
	import ToolCallCard from '$lib/components/tool-call-card.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { isHosted } from '$lib/ipc.svelte';

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
				if (!cancelled) error = String(cause);
			})
			.finally(() => {
				if (!cancelled) loading = false;
			});

		return () => {
			cancelled = true;
		};
	});

	async function loadMore() {
		if (!page) return;
		const next = await api.getTranscript(sessionId, page.turns.length, PAGE);
		page = { ...next, turns: [...page.turns, ...next.turns], offset: 0 };
	}

	function formatTime(iso: string | null): string {
		if (!iso) return '';
		const date = new Date(iso);
		return Number.isNaN(date.getTime())
			? ''
			: date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
	}

	// The condition must match what the markup actually renders. Testing
	// `turn.thinking !== null` regardless of the toggle let thinking-only turns
	// through as a bare timestamp with nothing under it.
	const visible = $derived(
		page
			? page.turns.filter(
					(turn) =>
						turn.text !== null ||
						(showThinking && turn.thinking !== null) ||
						(showTools && turn.toolCalls.length > 0)
				)
			: []
	);
</script>

<div class="flex h-full flex-col">
	<div class="flex shrink-0 flex-wrap items-center gap-2 border-b px-6 py-3">
		<Button
			variant={showTools ? 'default' : 'outline'}
			size="sm"
			class="h-7 px-2 text-xs font-normal"
			onclick={() => (showTools = !showTools)}
		>
			Tools
		</Button>
		<Button
			variant={showThinking ? 'default' : 'outline'}
			size="sm"
			class="h-7 px-2 text-xs font-normal"
			onclick={() => (showThinking = !showThinking)}
		>
			Thinking
		</Button>

		{#if page}
			<span class="ml-auto text-xs text-muted-foreground tabular-nums">
				{page.turns.length} of {page.total} turns
			</span>
		{/if}
	</div>

	<div class="min-h-0 flex-1 overflow-auto">
		<div class="mx-auto flex max-w-3xl flex-col gap-4 p-6">
			{#if !isHosted}
				<p class="text-sm text-muted-foreground">No host attached.</p>
			{:else if loading}
				{#each [...Array(5).keys()] as index (index)}
					<Skeleton class="h-20 w-full" />
				{/each}
			{:else if error}
				<Card.Root>
					<Card.Header>
						<Card.Title>Could not read the transcript</Card.Title>
						<Card.Description class="font-mono text-xs">{error}</Card.Description>
					</Card.Header>
				</Card.Root>
			{:else if page && page.path === null}
				<Card.Root>
					<Card.Header>
						<Card.Title>No transcript on disk</Card.Title>
						<Card.Description>
							This session has figures but its file is no longer where it was scanned from.
						</Card.Description>
					</Card.Header>
				</Card.Root>
			{:else if visible.length === 0}
				<p class="text-sm text-muted-foreground">Nothing to show with the current filters.</p>
			{:else}
				{#each visible as turn (turn.index)}
					<div class="flex flex-col gap-2">
						<div class="flex items-center gap-2 text-xs text-muted-foreground">
							<span class="font-medium {turn.role === 'user' ? 'text-foreground' : ''}">
								{turn.role === 'user' ? 'You' : 'Claude'}
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
									Thinking
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
						Load {Math.min(PAGE, page.total - page.turns.length)} more
					</Button>
				{/if}
			{/if}
		</div>
	</div>
</div>
