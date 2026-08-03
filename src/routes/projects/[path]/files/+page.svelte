<script lang="ts">
	import { page } from '$app/state';
	import ProjectFiles from '$lib/views/project-files.svelte';
</script>

<ProjectFiles path={page.params.path ?? ''} />
