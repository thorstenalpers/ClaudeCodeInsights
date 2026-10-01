<script lang="ts">
	import { page } from '$app/state';
	import ProjectSessions from '$lib/views/project-sessions.svelte';
</script>

<ProjectSessions path={page.params.path ?? ''} />
