<script lang="ts">
	import Download from '@lucide/svelte/icons/download';
	import Trash from '@lucide/svelte/icons/trash-2';
	import ResetView from '$lib/components/reset-view.svelte';
	import SortHeader from '$lib/components/sort-header.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import * as Table from '$lib/components/ui/table';
	import { t } from '$lib/i18n/index.svelte';
	import { logs, type LogLevel, type LogLine } from '$lib/logs.svelte';
	import { createTable } from '$lib/table.svelte';

	type Which = 'both' | 'host' | 'app';
	/** A line with the half it came from, which only the mixed view shows. */
	type Entry = LogLine & { from: 'host' | 'app' };

	// Mixed first and by default: a download that fails in the host and the
	// window's answer to it are one story, and reading it in two tabs means
	// holding the clock in your head.
	let which = $state<Which>('both');

	const merged = $derived.by<Entry[]>(() => {
		const host: Entry[] = logs.host.map((line) => ({ ...line, from: 'host' }));
		const app: Entry[] = logs.app.map((line) => ({ ...line, from: 'app' }));
		return [...host, ...app].sort((a, b) => a.at - b.at);
	});

	const chosen = $derived<Entry[]>(
		which === 'both' ? merged : merged.filter((line) => line.from === which)
	);

	/** Which of the two the line came from, named for the language it is in. */
	const ORIGIN: Record<'host' | 'app', string> = { host: 'rust', app: 'svelte' };

	// Level sorts by severity, not by name: a reader asking for the worst first
	// does not want "debug, error, info, warn".
	const RANK: Record<LogLevel, number> = { debug: 0, info: 1, warn: 2, error: 3 };

	const COLUMNS = [
		{ id: 'at', label: 'logs.column.time' as const },
		{ id: 'from', label: 'logs.column.origin' as const, class: 'hidden @md:table-cell' },
		{ id: 'level', label: 'logs.column.level' as const },
		{ id: 'source', label: 'logs.column.source' as const, class: 'hidden @lg:table-cell' },
		{ id: 'message', label: 'logs.column.message' as const }
	];

	const CLASS: Record<string, string> = Object.fromEntries(
		COLUMNS.map((column) => [column.id, 'class' in column ? (column.class ?? '') : ''])
	);

	const table = createTable<Entry>(
		() => chosen,
		{
			at: (line) => line.at,
			// Filtered on the words the column shows, sorted on what they mean.
			from: (line) => ORIGIN[line.from],
			level: (line) => line.level,
			source: (line) => line.source,
			message: (line) => line.message
		},
		{ sort: 'at', descending: false }
	);

	const rows = $derived(
		table.sorts.some((entry) => entry.id === 'level')
			? [...table.rows].sort((a, b) => {
					const entry = table.sorts.find((sort) => sort.id === 'level');
					const by = RANK[a.level] - RANK[b.level];
					return entry?.descending ? -by : by;
				})
			: table.rows
	);

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

	/** What is on screen as text, for pasting into a report. */
	function save(): void {
		const body = rows
			.map(
				(line) =>
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

<div class="@container flex h-full min-h-0 flex-col gap-4 p-4">
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

				<Input placeholder={t('common.search')} class="h-8 max-w-xs" bind:value={table.query} />
				<ResetView show={table.dirty} onreset={() => table.reset()} />

				<Badge variant="secondary" class="ml-auto">{rows.length}</Badge>
				<Button variant="outline" size="sm" onclick={save} disabled={rows.length === 0}>
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

		<Card.Content
			class="min-h-0 flex-1 overflow-auto [&_td]:py-1 [&_td]:font-mono [&_td]:text-xs [&_th]:h-8"
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
					{#each rows as line, index (index)}
						<Table.Row>
							<Table.Cell class="whitespace-nowrap text-muted-foreground/60">
								{clock(line.at)}
							</Table.Cell>
							<Table.Cell class={[CLASS.from, 'text-muted-foreground/60']}>
								{ORIGIN[line.from]}
							</Table.Cell>
							<Table.Cell class="uppercase {TONE[line.level]}">{line.level}</Table.Cell>
							<Table.Cell class={[CLASS.source, 'max-w-56 truncate text-muted-foreground']}>
								{line.source}
							</Table.Cell>
							<Table.Cell class="wrap-anywhere">{line.message}</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>

			{#if rows.length === 0}
				<p class="py-8 text-center text-sm text-muted-foreground">{t('logs.empty')}</p>
			{/if}
		</Card.Content>
	</Card.Root>
</div>
