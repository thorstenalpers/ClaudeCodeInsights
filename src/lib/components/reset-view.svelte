<script lang="ts">
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import { Button } from '$lib/components/ui/button';
	import { t } from '$lib/i18n/index.svelte';

	// Hidden while there is nothing to undo: a control that does nothing is a
	// question the user has to answer every time they look at the toolbar.
	let { show, onreset }: { show: boolean; onreset: () => void } = $props();
</script>

{#if show}
	<Button
		variant="ghost"
		size="sm"
		class="h-8 shrink-0 gap-1 px-2 text-xs font-normal"
		title={t('common.resetViewHint')}
		onclick={onreset}
	>
		<RotateCcw class="size-3.5" />
		{t('common.resetView')}
	</Button>
{/if}
