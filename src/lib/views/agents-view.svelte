<script lang="ts">
	import Icon from '@lucide/svelte/icons/bot';
	import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
</script>

<PlaceholderPanel
	icon={Icon}
	title="Agents"
	description="Subagent runs and what they cost"
	planned={['Runs grouped by agent type', 'Duration, tool calls and delegation share']}
/>
