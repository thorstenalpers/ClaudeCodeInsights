<script lang="ts">
	/**
	 * Folds the rail away and back, and says which of the two it will do.
	 *
	 * The shadcn trigger shows one icon for both states, which leaves the reader
	 * guessing; this one points the way it is about to move.
	 */
	import PanelLeftClose from '@lucide/svelte/icons/panel-left-close';
	import PanelLeftOpen from '@lucide/svelte/icons/panel-left-open';
	import { Button } from '$lib/components/ui/button';
	import { useSidebar } from '$lib/components/ui/sidebar';
	import { t } from '$lib/i18n/index.svelte';
	import { cn } from '$lib/utils';

	type Props = { class?: string };
	let { class: className }: Props = $props();

	const sidebar = useSidebar();

	const open = $derived(sidebar.isMobile ? sidebar.openMobile : sidebar.state === 'expanded');
</script>

<Button
	variant="ghost"
	size="icon-sm"
	class={cn('shrink-0', className)}
	aria-label={t('nav.rail')}
	title={t('nav.rail')}
	aria-expanded={open}
	onclick={() => sidebar.toggle()}
>
	{#if open}
		<PanelLeftClose />
	{:else}
		<PanelLeftOpen />
	{/if}
</Button>
