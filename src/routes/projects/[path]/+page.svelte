<script lang="ts">
	import { page } from '$app/state';
	import ProjectOverview from '$lib/views/project-overview.svelte';
</script>

<ProjectOverview path={page.params.path ?? ''} />
