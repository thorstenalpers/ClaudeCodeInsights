<script lang="ts">
	/**
	 * What every figure in the window rests on, said out loud.
	 *
	 * The same number means two different things depending on how the account
	 * is billed, and chatting costs nothing on a subscription but real money
	 * through an API key. Neither is visible from the numbers themselves, so the
	 * bar states both rather than leaving them to be inferred.
	 */
	import Cpu from '@lucide/svelte/icons/cpu';
	import CreditCard from '@lucide/svelte/icons/credit-card';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import { LOCAL_SOURCE, assistant } from '$lib/assistant.svelte';
	import { t } from '$lib/i18n/index.svelte';
	import { billing, plan } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';

	const subscription = $derived(billing.mode === 'subscription');
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
	<span class="flex shrink-0 items-center gap-1.5" title={t('status.billing.hint')}>
		{#if subscription}
			<CreditCard class="size-3.5 text-primary" />
			<span class="font-medium text-foreground">{t(`cost.plan.${plan.id}`)}</span>
			<span class="hidden @md:inline">{t('status.billing.subscription')}</span>
		{:else}
			<KeyRound class="size-3.5" />
			<span class="font-medium text-foreground">{t('status.billing.api')}</span>
			<span class="hidden @md:inline">{t('status.billing.api.hint')}</span>
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
