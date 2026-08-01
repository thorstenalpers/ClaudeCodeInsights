<script lang="ts">
  import Icon from '@lucide/svelte/icons/messages-square';
  import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
</script>

<PlaceholderPanel
  icon={Icon}
  title="Sessions"
  description="Every conversation, with its tokens and cost"
  planned={[
    'Sortable, filterable table of all sessions',
    'Derived activity per session, plus your own tags',
    'Click through to the full transcript',
  ]}
/>
