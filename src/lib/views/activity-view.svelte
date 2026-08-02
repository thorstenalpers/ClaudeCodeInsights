<script lang="ts">
	import Icon from '@lucide/svelte/icons/calendar-clock';
	import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
	import { t } from '$lib/i18n/index.svelte';
</script>

<PlaceholderPanel
	icon={Icon}
	title={t('nav.activity')}
	description={t('nav.activity.description')}
	planned={[t('placeholder.activity.1'), t('placeholder.activity.2')]}
/>
