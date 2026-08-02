<script lang="ts">
	import Icon from '@lucide/svelte/icons/wrench';
	import PlaceholderPanel from '$lib/components/placeholder-panel.svelte';
	import { t } from '$lib/i18n/index.svelte';
</script>

<PlaceholderPanel
	icon={Icon}
	title={t('nav.tools')}
	description={t('nav.tools.description')}
	planned={[t('placeholder.tools.1'), t('placeholder.tools.2')]}
/>
