<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import Languages from '@lucide/svelte/icons/languages';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { LOCALES, i18n, t, type LocaleSetting } from '$lib/i18n/index.svelte';

	const entries: { id: LocaleSetting; label: string }[] = $derived([
		{ id: 'system', label: t('settings.language.system') },
		...LOCALES.map((locale) => ({ id: locale.id, label: locale.label }))
	]);
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button {...props} variant="ghost" size="icon" aria-label={t('settings.language')}>
				<Languages />
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>

	<DropdownMenu.Content align="end" class="w-40">
		{#each entries as entry (entry.id)}
			<DropdownMenu.Item onSelect={() => i18n.set(entry.id)}>
				<span class="flex-1">{entry.label}</span>
				{#if i18n.setting === entry.id}
					<Check class="size-4" />
				{/if}
			</DropdownMenu.Item>
		{/each}
	</DropdownMenu.Content>
</DropdownMenu.Root>
