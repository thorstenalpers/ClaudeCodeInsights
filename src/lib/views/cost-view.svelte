<script lang="ts">
	import Icon from '@lucide/svelte/icons/circle-dollar-sign';
	import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
	import { t } from '$lib/i18n/index.svelte';
</script>

<PlaceholderPanel
	icon={Icon}
	title={t('nav.cost')}
	description={t('nav.cost.description')}
	planned={[t('placeholder.cost.1'), t('placeholder.cost.2')]}
/>
