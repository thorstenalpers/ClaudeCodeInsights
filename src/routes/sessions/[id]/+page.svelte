<script lang="ts">
	import { page } from '$app/state';
	import SessionDetail from '$lib/views/session-detail.svelte';
</script>

<SessionDetail sessionId={page.params.id!} />
