<script lang="ts">
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import { Button } from '$lib/components/ui/button';
	import { t } from '$lib/i18n/index.svelte';
	import { isHosted } from '$lib/ipc.svelte';
	import { scan } from '$lib/scan.svelte';
</script>

{#if isHosted}
	<div class="flex items-center gap-2">
		{#if scan.status === 'running'}
			<span class="text-xs text-muted-foreground tabular-nums">
				{scan.filesDone}/{scan.filesTotal}
			</span>
		{:else if scan.status === 'done' && scan.stats}
			<span class="text-xs text-muted-foreground tabular-nums">
				{t('header.sessionsScanned', { count: scan.stats.sessionsSeen })}
			</span>
		{/if}

		<Button
			variant="ghost"
			size="icon"
			aria-label={t('header.rescan')}
			disabled={scan.status === 'running'}
			onclick={() => scan.start()}
		>
			<RefreshCw class={scan.status === 'running' ? 'animate-spin' : undefined} />
		</Button>
	</div>
{/if}
