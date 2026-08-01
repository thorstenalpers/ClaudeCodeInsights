<script lang="ts">
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import { Button } from '$lib/components/ui/button';
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
				{scan.stats.sessionsSeen} sessions
			</span>
		{/if}

		<Button
			variant="ghost"
			size="icon"
			aria-label="Rescan transcripts"
			disabled={scan.status === 'running'}
			onclick={() => scan.start()}
		>
			<RefreshCw class={scan.status === 'running' ? 'animate-spin' : undefined} />
		</Button>
	</div>
{/if}
