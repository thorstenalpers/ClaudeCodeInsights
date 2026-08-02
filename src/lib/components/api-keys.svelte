<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import Gift from '@lucide/svelte/icons/gift';
	import { api, type ProviderInfo } from '$lib/api';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { openUrl } from '@tauri-apps/plugin-opener';

	let providers = $state<ProviderInfo[]>([]);
	let stored = $state<Record<string, boolean>>({});
	let saved = $state<Record<string, boolean>>({});
	let error = $state<string | null>(null);

	$effect(() => {
		if (!isHosted) return;
		void api.listProviders().then(async (list) => {
			providers = list;
			const flags: Record<string, boolean> = {};
			for (const provider of list) flags[provider.id] = await api.hasApiKey(provider.id);
			stored = flags;
		});
	});

	async function save(provider: string, value: string) {
		error = null;
		try {
			await api.setApiKey(provider, value);
			stored = { ...stored, [provider]: value.trim() !== '' };
			// The field is cleared either way: the key never comes back from the
			// store, so leaving it on screen would only pretend it did.
			saved = { ...saved, [provider]: true };
			setTimeout(() => (saved = { ...saved, [provider]: false }), 2000);
		} catch (cause) {
			error = errorMessage(cause);
		}
	}

	function onKeyInput(provider: string, event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const value = input.value;
		input.value = '';
		void save(provider, value);
	}
</script>

<div class="flex flex-col gap-2">
	{#each providers as provider (provider.id)}
		<div class="flex flex-wrap items-center gap-2">
			<span class="w-24 shrink-0 text-xs font-medium capitalize">{provider.id}</span>

			<Input
				type="password"
				class="h-8 max-w-64 font-mono text-xs"
				placeholder={t('settings.keys.placeholder')}
				onchange={(event: Event) => onKeyInput(provider.id, event)}
			/>

			{#if saved[provider.id]}
				<Badge variant="secondary" class="gap-1 font-normal">
					<Check class="size-3" />
					{t('settings.keys.saved')}
				</Badge>
			{:else if stored[provider.id]}
				<Badge variant="secondary" class="font-normal">{t('settings.keys.stored')}</Badge>
			{:else}
				<span class="text-xs text-muted-foreground">{t('settings.keys.none')}</span>
			{/if}

			{#if provider.freeKeyUrl}
				{@const url = provider.freeKeyUrl}
				<Button
					variant="ghost"
					size="sm"
					class="h-7 gap-1 px-2 text-xs font-normal"
					onclick={() => void openUrl(url)}
				>
					<Gift class="size-3.5" />
					{t('settings.keys.free')}
					<ExternalLink class="size-3 opacity-60" />
				</Button>
			{/if}
		</div>
	{/each}

	{#if error}
		<p class="text-xs text-destructive">{error}</p>
	{/if}
</div>
