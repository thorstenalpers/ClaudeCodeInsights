<script lang="ts">
	import Send from '@lucide/svelte/icons/send';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import {
		api,
		type CliStatus,
		type ModelRow,
		type ProviderInfo,
		type Rhythm,
		type ToolRow
	} from '$lib/api';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { assistant, LOCAL_SOURCE } from '$lib/assistant.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Textarea } from '$lib/components/ui/textarea';
	import { cli } from '$lib/cli.svelte';
	import { exact } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { costOf } from '$lib/pricing.svelte';
	import { scan } from '$lib/scan.svelte';

	let status = $state<CliStatus | null>(null);
	let providers = $state<ProviderInfo[]>([]);
	let models = $state<ModelRow[]>([]);
	let tools = $state<ToolRow[]>([]);
	let rhythm = $state<Rhythm | null>(null);

	let question = $state('');
	let answer = $state<string | null>(null);
	let error = $state<string | null>(null);
	let busy = $state(false);

	$effect(() => {
		if (!isHosted) return;
		void cli.path;
		void api.getCliStatus(cli.configured).then((value) => (status = value));
		void api.listProviders().then((value) => (providers = value));
	});

	const usingLocal = $derived(assistant.source === LOCAL_SOURCE);

	const sourceLabel = $derived(
		usingLocal
			? t('assistant.source.local')
			: (providers.find((entry) => entry.id === assistant.source)?.id ?? assistant.source)
	);

	$effect(() => {
		void scan.dataVersion;
		if (!isHosted) return;
		void api.listModels().then((value) => (models = value));
		void api.listTools().then((value) => (tools = value));
		void api.getRhythm().then((value) => (rhythm = value));
	});

	/**
	 * The figures, as plain text.
	 *
	 * Deliberately a summary rather than the database: the question is about
	 * totals, and a transcript would be both useless here and the one thing this
	 * app promises never to hand around.
	 */
	const context = $derived.by(() => {
		const lines: string[] = ['Claude Code usage on this machine.', ''];

		if (models.length > 0) {
			lines.push('Models (turns, input, output, cache read, cost at API rates in USD):');
			for (const row of models) {
				lines.push(
					`- ${row.model}: ${exact(row.turns)} turns, ${exact(row.inputTokens)} in, ` +
						`${exact(row.outputTokens)} out, ${exact(row.cacheReadTokens)} cache read, ` +
						`$${costOf(row.model, row).toFixed(2)}`
				);
			}
			lines.push('');
		}

		if (tools.length > 0) {
			lines.push('Top tools (calls, sessions):');
			for (const row of tools.slice(0, 10)) {
				lines.push(`- ${row.name}: ${exact(row.calls)} calls in ${exact(row.sessions)} sessions`);
			}
			lines.push('');
		}

		if (rhythm) {
			lines.push(
				`Active days: ${rhythm.activeDays}, longest streak ${rhythm.longestStreak}, ` +
					`current streak ${rhythm.currentStreak}.`
			);
		}

		return lines.join('\n');
	});

	const suggestions = $derived([
		t('assistant.suggestion.1'),
		t('assistant.suggestion.2'),
		t('assistant.suggestion.3')
	]);

	async function ask() {
		if (!question.trim() || busy) return;
		busy = true;
		error = null;
		answer = null;
		try {
			answer = await api.askClaude(
				assistant.source,
				cli.configured,
				`${context}\n\nQuestion: ${question.trim()}\n\nAnswer briefly, using only the figures above.`
			);
		} catch (cause) {
			error = errorMessage(cause);
		} finally {
			busy = false;
		}
	}
</script>

<div class="flex h-full flex-col gap-4 overflow-auto p-6">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if usingLocal && status && !status.found}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('assistant.missing')}</Card.Title>
				<Card.Description>{t('assistant.missingBody')}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else}
		<div class="flex flex-wrap items-center gap-2">
			<Sparkles class="size-4 shrink-0 text-muted-foreground" />

			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="outline" size="sm" class="h-8 gap-1 font-normal capitalize">
							{sourceLabel}
							<ChevronDown class="size-3.5 opacity-60" />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content align="start" class="w-56">
					<DropdownMenu.Item onSelect={() => assistant.set(LOCAL_SOURCE)}>
						<span class="flex-1">{t('assistant.source.local')}</span>
						{#if usingLocal}
							<Check class="size-4" />
						{/if}
					</DropdownMenu.Item>
					<DropdownMenu.Separator />
					{#each providers as provider (provider.id)}
						<DropdownMenu.Item onSelect={() => assistant.set(provider.id)}>
							<span class="flex flex-1 flex-col">
								<span class="capitalize">{provider.id}</span>
								<span class="text-xs text-muted-foreground">{provider.model}</span>
							</span>
							{#if assistant.source === provider.id}
								<Check class="size-4" />
							{/if}
						</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Content>
			</DropdownMenu.Root>

			<span class="text-xs text-muted-foreground">
				{#if usingLocal && status?.path}
					{t('assistant.found', { path: status.path })}
				{:else if !usingLocal}
					{t('assistant.sendsData')}
				{/if}
			</span>
		</div>

		<div class="flex flex-wrap gap-2">
			{#each suggestions as suggestion (suggestion)}
				<Button
					variant="outline"
					size="sm"
					class="h-8 font-normal"
					onclick={() => (question = suggestion)}
				>
					{suggestion}
				</Button>
			{/each}
		</div>

		<div class="flex flex-col gap-2">
			<Textarea
				bind:value={question}
				placeholder={t('assistant.placeholder')}
				class="min-h-24"
				disabled={busy}
			/>
			<Button class="self-end" disabled={busy || !question.trim()} onclick={ask}>
				<Send class="size-4" />
				{busy ? t('assistant.asking') : t('assistant.ask')}
			</Button>
		</div>

		{#if error}
			<Card.Root>
				<Card.Header>
					<Card.Title>{t('assistant.failed')}</Card.Title>
					<Card.Description class="font-mono text-xs">{error}</Card.Description>
				</Card.Header>
			</Card.Root>
		{/if}

		{#if answer}
			<Card.Root>
				<Card.Content class="pt-6 text-sm leading-relaxed whitespace-pre-wrap">
					{answer}
				</Card.Content>
			</Card.Root>
		{/if}

		<details class="rounded-md border p-3 text-xs">
			<summary class="cursor-pointer text-muted-foreground">{t('assistant.context')}</summary>
			<p class="mt-2 text-muted-foreground">{t('assistant.contextNote')}</p>
			<pre class="mt-2 overflow-x-auto font-mono text-[11px] whitespace-pre-wrap">{context}</pre>
		</details>
	{/if}
</div>
