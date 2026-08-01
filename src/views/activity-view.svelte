<script lang="ts">
  import Icon from '@lucide/svelte/icons/calendar-clock';
  import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
</script>

<PlaceholderPanel
  icon={Icon}
  title="Activity"
  description="When the work actually happens"
  planned={[
    'Weekday by hour heatmap',
    'Session length distribution and streaks',
  ]}
/>
