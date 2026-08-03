<script lang="ts">
	import Mic from '@lucide/svelte/icons/mic';
	import Square from '@lucide/svelte/icons/square';
	import Send from '@lucide/svelte/icons/send';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import { api, type CliStatus, type ProviderInfo } from '$lib/api';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { assistant, LOCAL_SOURCE } from '$lib/assistant.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Textarea } from '$lib/components/ui/textarea';
	import { cli } from '$lib/cli.svelte';
	import { displayPath } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';
	import { voice } from '$lib/voice.svelte';

	let status = $state<CliStatus | null>(null);
	let providers = $state<ProviderInfo[]>([]);
	let models = $state<string[]>([]);
	let efforts = $state<string[]>([]);

	let question = $state('');
	let answer = $state<string | null>(null);
	let error = $state<string | null>(null);
	let busy = $state(false);

	$effect(() => {
		if (!isHosted) return;
		void cli.path;
		void api.getCliStatus(cli.configured).then((value) => (status = value));
		void api.listProviders().then((value) => (providers = value));
		void api.localOptions().then(([m, e]) => {
			models = m;
			efforts = e;
		});
	});

	const usingLocal = $derived(assistant.source === LOCAL_SOURCE);

	const sourceLabel = $derived(
		usingLocal
			? t('assistant.source.local')
			: (providers.find((entry) => entry.id === assistant.source)?.id ?? assistant.source)
	);

	$effect(() => {
		void scan.dataVersion;
		void assistant.load();
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
			answer = await assistant.ask(question);
			if (voice.speaks) void voice.speak(answer);
		} catch (cause) {
			error = errorMessage(cause);
		} finally {
			busy = false;
		}
	}

	// A spoken sentence is a command as often as a question, so it is routed
	// before it is answered; a recognised command asks via toast first.
	async function dictate() {
		const heard = await voice.listen();
		if (!heard) return;

		busy = true;
		try {
			const routed = await assistant.obey(heard);
			if (routed.clarify) {
				question = heard;
				answer = routed.clarify;
				if (voice.speaks) void voice.speak(routed.clarify);
			}
			if (routed.handled) return;
		} catch (cause) {
			error = errorMessage(cause);
			return;
		} finally {
			busy = false;
		}

		question = heard;
		await ask();
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

			{#if usingLocal}
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="outline" size="sm" class="h-8 gap-1 font-normal">
								{assistant.model || t('assistant.model.default')}
								<ChevronDown class="size-3.5 opacity-60" />
							</Button>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="start" class="w-48">
						<DropdownMenu.Item onSelect={() => assistant.setModel('')}>
							<span class="flex-1">{t('assistant.model.default')}</span>
							{#if assistant.model === ''}<Check class="size-4" />{/if}
						</DropdownMenu.Item>
						{#each models as option (option)}
							<DropdownMenu.Item onSelect={() => assistant.setModel(option)}>
								<span class="flex-1 capitalize">{option}</span>
								{#if assistant.model === option}<Check class="size-4" />{/if}
							</DropdownMenu.Item>
						{/each}
					</DropdownMenu.Content>
				</DropdownMenu.Root>

				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="outline" size="sm" class="h-8 gap-1 font-normal">
								{assistant.effort || t('assistant.effort.default')}
								<ChevronDown class="size-3.5 opacity-60" />
							</Button>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="start" class="w-48">
						<DropdownMenu.Label class="text-xs font-normal text-muted-foreground">
							{t('assistant.effort')}
						</DropdownMenu.Label>
						<DropdownMenu.Item onSelect={() => assistant.setEffort('')}>
							<span class="flex-1">{t('assistant.effort.default')}</span>
							{#if assistant.effort === ''}<Check class="size-4" />{/if}
						</DropdownMenu.Item>
						{#each efforts as option (option)}
							<DropdownMenu.Item onSelect={() => assistant.setEffort(option)}>
								<span class="flex-1">{option}</span>
								{#if assistant.effort === option}<Check class="size-4" />{/if}
							</DropdownMenu.Item>
						{/each}
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			{/if}

			<span class="text-xs text-muted-foreground">
				{#if usingLocal && status?.path}
					{t('assistant.found', { path: displayPath(status.path) })}
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
			<div class="flex flex-wrap items-center justify-end gap-2">
				<label class="mr-auto flex items-center gap-2 text-sm">
					<input
						type="checkbox"
						class="size-4 accent-primary"
						checked={voice.speaks}
						onchange={(event: Event) =>
							voice.setSpeaks((event.currentTarget as HTMLInputElement).checked)}
					/>
					{t('settings.voice.output')}
				</label>

				{#if voice.speaking}
					<Button variant="outline" onclick={() => voice.silence()}>
						<Square class="size-4" />
						{t('voice.stop')}
					</Button>
				{/if}

				<Button
					variant="outline"
					disabled={busy || voice.listening || voice.available === false}
					title={voice.available === false ? t('settings.voice.unavailable') : t('voice.listen')}
					onclick={dictate}
				>
					<Mic class={['size-4', voice.listening && 'text-destructive']} />
					{voice.listening ? t('voice.listening') : t('voice.listen')}
				</Button>
				<Button disabled={busy || !question.trim()} onclick={ask}>
					<Send class="size-4" />
					{busy ? t('assistant.asking') : t('assistant.ask')}
				</Button>
			</div>

			{#if voice.needsPrivacy}
				<p class="rounded-md border border-amber-500/40 p-2 text-xs">{t('voice.privacy')}</p>
			{:else if voice.error}
				<p class="text-xs text-destructive">{t('voice.failed', { message: voice.error })}</p>
			{/if}
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
				<Card.Content class="flex flex-col gap-2 pt-6">
					<p class="text-sm leading-relaxed whitespace-pre-wrap">{answer}</p>
					{#if assistant.last}
						<!-- Reported by the source, not repeated back from the request:
						     an alias can resolve to a different model than expected. -->
						<p class="flex flex-wrap gap-x-3 text-xs text-muted-foreground">
							{#if assistant.last.model}
								<span>{t('assistant.answeredBy', { model: assistant.last.model })}</span>
							{/if}
							{#if assistant.last.effort}
								<span>{t('assistant.effort')}: {assistant.last.effort}</span>
							{/if}
							{#if assistant.last.costUsd !== null}
								<span
									>{t('assistant.answerCost', {
										amount: region.format(assistant.last.costUsd)
									})}</span
								>
							{/if}
						</p>
					{/if}
				</Card.Content>
			</Card.Root>
		{/if}

		<details class="rounded-md border p-3 text-xs">
			<summary class="cursor-pointer text-muted-foreground">{t('assistant.context')}</summary>
			<p class="mt-2 text-muted-foreground">{t('assistant.contextNote')}</p>
			<pre
				class="mt-2 overflow-x-auto font-mono text-[11px] whitespace-pre-wrap">{assistant.context}</pre>
		</details>
	{/if}
</div>
