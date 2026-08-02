<script lang="ts">
	import Icon from '@lucide/svelte/icons/sparkles';
	import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
</script>

<PlaceholderPanel
	icon={Icon}
	title="Assistant"
	description="Ask questions about your own usage"
	planned={['Streaming chat over the local database', 'Answers built from tools, never invented']}
/>
