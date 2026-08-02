<script lang="ts">
	import Icon from '@lucide/svelte/icons/bot';
	import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
	import { t } from '$lib/i18n/index.svelte';
</script>

<PlaceholderPanel
	icon={Icon}
	title={t('nav.agents')}
	description={t('nav.agents.description')}
	planned={[t('placeholder.agents.1'), t('placeholder.agents.2')]}
/>
