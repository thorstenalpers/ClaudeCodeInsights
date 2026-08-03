<script lang="ts">
	import { page } from '$app/state';
	import ProjectTime from '$lib/views/project-time.svelte';
</script>

<ProjectTime path={page.params.path ?? ''} />
