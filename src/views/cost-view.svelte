<script lang="ts">
  import Icon from '@lucide/svelte/icons/circle-dollar-sign';
  import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
</script>

<PlaceholderPanel
  icon={Icon}
  title="Cost & Models"
  description="What each model costs, and why"
  planned={[
    'Cost split across the four token classes',
    'Editable price table, unknown models shown as n/a',
  ]}
/>
