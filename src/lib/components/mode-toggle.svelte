<script lang="ts">
	import Moon from '@lucide/svelte/icons/moon';
	import Sun from '@lucide/svelte/icons/sun';
	import { Button } from '$lib/components/ui/button';
	import { t } from '$lib/i18n/index.svelte';
	import { theme } from '$lib/theme.svelte';

	// The header carries the one switch worth reaching for constantly. Preset and
	// brand colours are set once and live in Settings.
	const isDark = $derived(theme.resolvedMode === 'dark');
</script>

<Button
	variant="ghost"
	size="icon"
	aria-label={isDark ? t('header.toLight') : t('header.toDark')}
	onclick={() => theme.setMode(isDark ? 'light' : 'dark')}
>
	{#if isDark}
		<Sun />
	{:else}
		<Moon />
	{/if}
</Button>
