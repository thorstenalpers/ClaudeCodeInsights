<script lang="ts">
	import { nav } from '$lib/nav.svelte';
	import { ALL_PAGES } from '$lib/pages';
	import SessionDetail from '../../views/session-detail.svelte';
</script>

<!--
  Every visited page stays mounted; switching only toggles visibility.

  The `{#if}` is around the *mount set*, not around the active key. Putting it
  around the active key would destroy and recreate the page on every switch,
  throwing away scroll position, table sorting and chat scrollback — the exact
  state a desktop app is expected to keep. The cost is that a page's first data
  fetch happens on its first visit rather than at startup, which is also what
  keeps startup fast.
-->
{#each ALL_PAGES as page (page.key)}
	{#if nav.mounted.has(page.key)}
		<div
			class="absolute inset-0 overflow-auto"
			data-page={page.key}
			hidden={nav.active !== page.key || nav.detailSessionId !== null}
		>
			<page.component />
		</div>
	{/if}
{/each}

<!-- Detail covers the page instead of replacing it, so going back returns to a
     table that never lost its scroll position or its filters. -->
{#if nav.detailSessionId}
	<div class="absolute inset-0 bg-background" data-page="session-detail">
		{#key nav.detailSessionId}
			<SessionDetail sessionId={nav.detailSessionId} />
		{/key}
	</div>
{/if}
