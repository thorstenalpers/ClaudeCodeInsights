<script lang="ts">
	import Download from '@lucide/svelte/icons/download';
	import Trash from '@lucide/svelte/icons/trash-2';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { t } from '$lib/i18n/index.svelte';
	import { logs, type LogLevel, type LogLine } from '$lib/logs.svelte';

	type Which = 'both' | 'host' | 'app';
	/** A line with the half it came from, which only the mixed view shows. */
	type Entry = LogLine & { from: 'host' | 'app' };

	// Mixed first and by default: a download that fails in the host and the
	// window's answer to it are one story, and reading it in two tabs means
	// holding the clock in your head.
	let which = $state<Which>('both');
	let level = $state<LogLevel | 'all'>('all');

	const RANK: Record<LogLevel, number> = { debug: 0, info: 1, warn: 2, error: 3 };

	const merged = $derived.by<Entry[]>(() => {
		const host: Entry[] = logs.host.map((line) => ({ ...line, from: 'host' }));
		const app: Entry[] = logs.app.map((line) => ({ ...line, from: 'app' }));
		return [...host, ...app].sort((a, b) => a.at - b.at);
	});

	const chosen = $derived<Entry[]>(
		which === 'both' ? merged : merged.filter((line) => line.from === which)
	);

	const lines = $derived(
		chosen.filter((line) => level === 'all' || RANK[line.level] >= RANK[level])
	);

	/** Which of the two the line came from, named for the language it is in. */
	const ORIGIN: Record<'host' | 'app', string> = { host: 'rust', app: 'svelte' };

	const TONE: Record<LogLevel, string> = {
		error: 'text-destructive',
		warn: 'text-chart-3',
		info: 'text-muted-foreground',
		debug: 'text-muted-foreground/60'
	};

	function clock(at: number): string {
		return new Date(at).toLocaleTimeString();
	}

	function clear(): void {
		if (which !== 'app') logs.clear('host');
		if (which !== 'host') logs.clear('app');
	}

	/** The whole buffer as text, for pasting into a report. */
	function save(): void {
		const body = lines
			.map(
				(line: Entry) =>
					`${clock(line.at)} [${line.level}] ${ORIGIN[line.from]} ${line.source} ${line.message}`
			)
			.join('\n');
		const url = URL.createObjectURL(new Blob([body], { type: 'text/plain' }));
		const link = document.createElement('a');
		link.href = url;
		link.download = `claude-insights-${which}.log`;
		link.click();
		URL.revokeObjectURL(url);
	}
</script>

<div class="flex h-full min-h-0 flex-col gap-4 p-4">
	<Card.Root class="flex min-h-0 flex-1 flex-col">
		<Card.Header class="gap-3">
			<div class="flex flex-wrap items-center gap-2">
				<div class="flex rounded-md border p-0.5">
					{#each [{ id: 'both', label: 'logs.both' }, { id: 'host', label: 'logs.host' }, { id: 'app', label: 'logs.app' }] as const as tab (tab.id)}
						<Button
							variant={which === tab.id ? 'secondary' : 'ghost'}
							size="sm"
							onclick={() => (which = tab.id)}
						>
							{t(tab.label)}
						</Button>
					{/each}
				</div>

				<div class="flex rounded-md border p-0.5">
					{#each ['all', 'info', 'warn', 'error'] as const as step (step)}
						<Button
							variant={level === step ? 'secondary' : 'ghost'}
							size="sm"
							onclick={() => (level = step)}
						>
							{t(`logs.level.${step}` as 'logs.level.all')}
						</Button>
					{/each}
				</div>

				<Badge variant="secondary" class="ml-auto">{lines.length}</Badge>
				<Button variant="outline" size="sm" onclick={save} disabled={lines.length === 0}>
					<Download />
					{t('logs.save')}
				</Button>
				<Button variant="outline" size="sm" onclick={clear}>
					<Trash />
					{t('logs.clear')}
				</Button>
			</div>
			<Card.Description>
				{t(
					which === 'both'
						? 'logs.both.description'
						: which === 'host'
							? 'logs.host.description'
							: 'logs.app.description'
				)}
			</Card.Description>
		</Card.Header>

		<Card.Content class="min-h-0 flex-1 overflow-auto">
			{#if lines.length === 0}
				<p class="py-8 text-center text-sm text-muted-foreground">{t('logs.empty')}</p>
			{:else}
				<!-- Newest last, as a log reads; the container is scrolled by the user
				     rather than pinned, so a line being read does not slide away. -->
				<div class="flex flex-col gap-0.5 font-mono text-xs">
					{#each lines as line, index (index)}
						<div class="flex gap-2 wrap-anywhere">
							<span class="shrink-0 text-muted-foreground/60">{clock(line.at)}</span>
							{#if which === 'both'}
								<!-- Only where the two are mixed: elsewhere the tab says it. -->
								<span class="w-12 shrink-0 text-muted-foreground/60">{ORIGIN[line.from]}</span>
							{/if}
							<span class="w-12 shrink-0 uppercase {TONE[line.level]}">{line.level}</span>
							<span class="shrink-0 text-muted-foreground">{line.source}</span>
							<span class="min-w-0">{line.message}</span>
						</div>
					{/each}
				</div>
			{/if}
		</Card.Content>
	</Card.Root>
</div>
