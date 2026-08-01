<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import Palette from '@lucide/svelte/icons/palette';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { PRESETS, theme, type ThemeMode } from '$lib/theme.svelte';

	const MODES: { id: ThemeMode; label: string }[] = [
		{ id: 'light', label: 'Light' },
		{ id: 'dark', label: 'Dark' },
		{ id: 'system', label: 'System' }
	];
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button {...props} variant="ghost" size="icon" aria-label="Appearance">
				<Palette />
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>

	<DropdownMenu.Content align="end" class="w-44">
		<DropdownMenu.Label>Mode</DropdownMenu.Label>
		{#each MODES as m (m.id)}
			<DropdownMenu.Item onSelect={() => theme.setMode(m.id)}>
				<span class="flex-1">{m.label}</span>
				{#if theme.mode === m.id}
					<Check class="size-4" />
				{/if}
			</DropdownMenu.Item>
		{/each}

		<DropdownMenu.Separator />

		<DropdownMenu.Label>Colour</DropdownMenu.Label>
		{#each PRESETS as preset (preset.id)}
			<DropdownMenu.Item onSelect={() => theme.setPreset(preset.id)}>
				<span class="flex-1">{preset.label}</span>
				{#if theme.preset === preset.id}
					<Check class="size-4" />
				{/if}
			</DropdownMenu.Item>
		{/each}
	</DropdownMenu.Content>
</DropdownMenu.Root>
