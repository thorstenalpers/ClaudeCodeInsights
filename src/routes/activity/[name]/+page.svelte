<script lang="ts">
	import { page } from '$app/state';
	import ActivityDetail from '$lib/views/activity-detail.svelte';
</script>

<ActivityDetail activity={page.params.name ?? ''} />
