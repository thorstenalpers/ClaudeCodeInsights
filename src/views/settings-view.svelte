<script lang="ts">
  import Icon from '@lucide/svelte/icons/settings';
  import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
</script>

<PlaceholderPanel
  icon={Icon}
  title="Settings"
  description="Scan paths, appearance, API keys"
  planned={[
    'Transcript folders to scan',
    'Theme and colour preset',
    'API key per provider, stored in the OS keychain',
  ]}
/>
