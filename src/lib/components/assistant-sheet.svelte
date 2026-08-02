<script lang="ts">
	/**
	 * The assistant, reachable without leaving the page you are asking about.
	 *
	 * Built on the Sheet already in the project rather than a chat library:
	 * shadcn-svelte has no chat component, and a slide-over is the pattern it
	 * does offer — so this costs a file, not a dependency.
	 */
	import Mic from '@lucide/svelte/icons/mic';
	import Send from '@lucide/svelte/icons/send';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Volume2 from '@lucide/svelte/icons/volume-2';
	import { assistant } from '$lib/assistant.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Sheet from '$lib/components/ui/sheet';
	import { Textarea } from '$lib/components/ui/textarea';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { voice } from '$lib/voice.svelte';

	let open = $state(false);
	let question = $state('');
	let answer = $state<string | null>(null);
	let error = $state<string | null>(null);
	let busy = $state(false);

	$effect(() => {
		if (open) void assistant.load();
	});

	async function ask() {
		if (!question.trim() || busy) return;
		busy = true;
		error = null;
		answer = null;
		try {
			answer = await assistant.ask(question);
			if (voice.speaks) voice.speak(answer);
		} catch (cause) {
			error = errorMessage(cause);
		} finally {
			busy = false;
		}
	}

	async function dictate() {
		const heard = await voice.listen();
		if (heard) {
			question = question.trim() === '' ? heard : `${question.trim()} ${heard}`;
			await ask();
		}
	}
</script>

{#if isHosted}
	<Sheet.Root bind:open>
		<Sheet.Trigger>
			{#snippet child({ props })}
				<button
					{...props}
					type="button"
					aria-label={t('assistant.open')}
					title={t('assistant.open')}
					class="fixed right-4 bottom-10 z-30 flex size-11 items-center justify-center rounded-full border bg-background shadow-lg transition-colors hover:bg-accent"
				>
					<Sparkles class="size-5" />
				</button>
			{/snippet}
		</Sheet.Trigger>

		<Sheet.Content side="right" class="flex w-full flex-col gap-3 p-4 sm:max-w-md">
			<Sheet.Header class="p-0">
				<Sheet.Title>{t('nav.assistant')}</Sheet.Title>
				<Sheet.Description>{t('nav.assistant.description')}</Sheet.Description>
			</Sheet.Header>

			<div class="flex min-h-0 flex-1 flex-col gap-2 overflow-auto">
				{#if error}
					<p class="rounded-md border border-destructive/40 p-2 text-xs text-destructive">
						{error}
					</p>
				{/if}
				{#if answer}
					<div class="rounded-md border p-3 text-sm leading-relaxed whitespace-pre-wrap">
						{answer}
					</div>
				{/if}
				{#if voice.needsPrivacy}
					<p class="rounded-md border border-amber-500/40 p-2 text-xs">
						{t('voice.privacy')}
					</p>
				{:else if voice.error}
					<p class="text-xs text-destructive">{t('voice.failed', { message: voice.error })}</p>
				{/if}
			</div>

			<Textarea
				bind:value={question}
				placeholder={t('assistant.placeholder')}
				class="min-h-20 shrink-0"
				disabled={busy}
			/>

			<div class="flex shrink-0 items-center gap-2">
				{#if voice.mode === 'speech'}
					<Button
						variant="outline"
						size="sm"
						class="h-8 gap-1"
						disabled={busy || voice.listening}
						onclick={dictate}
					>
						<Mic class={['size-4', voice.listening && 'text-destructive']} />
						{voice.listening ? t('voice.listening') : t('voice.listen')}
					</Button>
				{/if}

				{#if answer}
					<Button
						variant="ghost"
						size="icon"
						class="size-8"
						aria-label={t('voice.speak')}
						onclick={() => voice.speak(answer ?? '')}
					>
						<Volume2 class="size-4" />
					</Button>
				{/if}

				<Button size="sm" class="ml-auto h-8" disabled={busy || !question.trim()} onclick={ask}>
					<Send class="size-4" />
					{busy ? t('assistant.asking') : t('assistant.ask')}
				</Button>
			</div>
		</Sheet.Content>
	</Sheet.Root>
{/if}
