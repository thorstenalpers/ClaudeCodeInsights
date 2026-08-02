<script lang="ts">
	import Icon from '@lucide/svelte/icons/sparkles';
	import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
	import { t } from '$lib/i18n/index.svelte';
</script>

<PlaceholderPanel
	icon={Icon}
	title={t('nav.assistant')}
	description={t('nav.assistant.description')}
	planned={[t('placeholder.assistant.1'), t('placeholder.assistant.2')]}
/>
