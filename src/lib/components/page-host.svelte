<script lang="ts">
  import { nav } from '$lib/nav.svelte';
  import { ALL_PAGES } from '$lib/pages';
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
      hidden={nav.active !== page.key}
    >
      <page.component />
    </div>
  {/if}
{/each}
