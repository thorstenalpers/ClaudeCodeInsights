<script lang="ts">
	import Icon from '@lucide/svelte/icons/wrench';
	import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
</script>

<PlaceholderPanel
	icon={Icon}
	title="Tools"
	description="Which tools consume the tokens"
	planned={['Tool calls and attributed tokens', 'Editable tool to category mapping']}
/>
