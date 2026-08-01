<script lang="ts">
  import Icon from '@lucide/svelte/icons/layout-dashboard';
  import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
</script>

<PlaceholderPanel
  icon={Icon}
  title="Overview"
  description="Tokens, cost and activity at a glance"
  planned={[
    'Sessions, turns, tokens and estimated cost',
    'Tokens over time, stacked by model',
    'Cache efficiency and top projects',
  ]}
/>
