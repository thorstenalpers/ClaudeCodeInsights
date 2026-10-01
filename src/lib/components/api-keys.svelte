<script lang="ts">
	/**
	 * One row per provider: pick it, see whether a key is stored, put one in.
	 *
	 * The key is written straight to the store and the field cleared, because
	 * nothing ever reads it back — leaving it on screen would only pretend the
	 * value came from somewhere.
	 */
	import Check from '@lucide/svelte/icons/check';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import Gift from '@lucide/svelte/icons/gift';
	import { api, type ProviderInfo } from '$lib/api';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { assistant } from '$lib/assistant.svelte';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { cn } from '$lib/utils';

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

<div class="flex flex-col divide-y divide-border/60">
	{#each providers as provider (provider.id)}
		<div class="flex flex-col gap-1.5 py-2 first:pt-0 last:pb-0">
			<div class="flex flex-wrap items-center gap-2">
				<!-- Choosing the provider here rather than in a second list: the row
				     that holds its key is where a reader looks for it. -->
				<button
					type="button"
					aria-pressed={assistant.source === provider.id}
					onclick={() => assistant.set(provider.id)}
					class={cn(
						'flex h-7 cursor-pointer items-center rounded-md border px-2.5 text-xs font-medium capitalize transition-colors',
						assistant.source === provider.id
							? 'border-primary/40 bg-primary/10 text-foreground'
							: 'border-border text-muted-foreground hover:bg-primary/10'
					)}
				>
					{provider.id}
				</button>
				<span class="truncate font-mono text-xs text-muted-foreground">{provider.model}</span>

				{#if saved[provider.id]}
					<Badge variant="secondary" class="ml-auto gap-1 font-normal">
						<Check class="size-3" />
						{t('settings.keys.saved')}
					</Badge>
				{:else if stored[provider.id]}
					<Badge variant="secondary" class="ml-auto font-normal">{t('settings.keys.stored')}</Badge>
				{:else}
					<span class="ml-auto text-xs text-muted-foreground">{t('settings.keys.none')}</span>
				{/if}
			</div>

			<div class="flex flex-wrap items-center gap-2">
				<Input
					type="password"
					class="h-7 max-w-64 flex-1 font-mono text-xs"
					placeholder={t('settings.keys.placeholder')}
					onchange={(event: Event) => onKeyInput(provider.id, event)}
				/>

				{#if stored[provider.id]}
					<Button
						variant="ghost"
						size="sm"
						class="h-7 px-2 text-xs font-normal"
						onclick={() => void save(provider.id, '')}
					>
						{t('settings.keys.forget')}
					</Button>
				{/if}

				{#if provider.freeKeyUrl}
					<Button
						variant="ghost"
						size="sm"
						class="h-7 gap-1 px-2 text-xs font-normal"
						onclick={() => void api.openFreeKeyUrl(provider.id)}
					>
						<Gift class="size-3.5" />
						{t('settings.keys.free')}
						<ExternalLink class="size-3 opacity-60" />
					</Button>
				{/if}
			</div>
		</div>
	{/each}

	{#if error}
		<p class="pt-2 text-xs text-destructive">{error}</p>
	{/if}
</div>
