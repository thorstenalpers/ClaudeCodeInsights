<script lang="ts">
	/**
	 * What every figure in the window rests on, said out loud.
	 *
	 * Which binary is being read, which model answered, and what the figures are
	 * converted at. None of that is visible from the numbers themselves, and a
	 * reader comparing two machines needs the version before the totals.
	 */
	import Cpu from '@lucide/svelte/icons/cpu';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import Terminal from '@lucide/svelte/icons/terminal';
	import { LOCAL_SOURCE, assistant } from '$lib/assistant.svelte';
	import { cli } from '$lib/cli.svelte';
	import { t } from '$lib/i18n/index.svelte';
	import { region } from '$lib/region.svelte';

	const chatsOnPlan = $derived(assistant.source === LOCAL_SOURCE);

	// What actually answered outranks what was asked for; before the first
	// answer there is only the request, and before that only the CLI's own
	// setting, which this app does not get to see.
	const model = $derived(assistant.last?.model || assistant.model || t('assistant.model.default'));
	const effort = $derived(
		assistant.last?.effort || assistant.effort || t('assistant.effort.default')
	);
</script>

<footer
	class="@container flex h-7 shrink-0 items-center gap-3 overflow-hidden border-t bg-muted/30 px-3 text-xs text-muted-foreground"
>
	<!-- The agent this window is reading and asking, named and versioned. Which
	     plan pays for it is a settings question; which binary answers is not. -->
	<span class="flex shrink-0 items-center gap-1.5" title={cli.status?.path ?? t('status.cli.hint')}>
		<Terminal class="size-3.5 {cli.status?.found ? 'text-primary' : 'text-muted-foreground'}" />
		<span class="font-medium text-foreground">{t('transcript.claude')}</span>
		{#if cli.status?.found}
			<span>{cli.status.version ?? t('status.cli.unknownVersion')}</span>
		{:else}
			<span class="hidden @md:inline">{t('status.cli.missing')}</span>
		{/if}
	</span>

	<span class="hidden shrink-0 items-center gap-1.5 @2xl:flex" title={t('status.chat.hint')}>
		<MessageSquare class="size-3.5" />
		{chatsOnPlan ? t('status.chat.local') : t('status.chat.hosted', { source: assistant.source })}
	</span>

	<span class="hidden shrink-0 items-center gap-1.5 @3xl:flex" title={t('status.model.hint')}>
		<Cpu class="size-3.5" />
		<span class="max-w-48 truncate">{model}</span>
		<span class="text-muted-foreground/70">· {effort}</span>
	</span>

	{#if region.isIndicative}
		<span class="ml-auto shrink-0 truncate" title={t('settings.rate.indicative.hint')}>
			{t('status.rate.indicative', { currency: region.currency, rate: region.rate })}
		</span>
	{/if}
</footer>
