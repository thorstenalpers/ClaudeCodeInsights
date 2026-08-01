<script lang="ts">
	import Icon from '@lucide/svelte/icons/folder-git-2';
	import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
</script>

<PlaceholderPanel
	icon={Icon}
	title="Projects"
	description="Usage per project and branch"
	planned={['Tokens, cost and sessions per project', 'Branch level breakdown']}
/>
