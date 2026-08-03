<script lang="ts">
	/**
	 * What a language change means for the voice, said once and only when it
	 * matters: reading aloud has to be switched on, or none of this is news.
	 *
	 * It draws nothing itself — the notice is a toast, so it can be raised from
	 * here and dismissed anywhere.
	 */
	import { toast } from 'svelte-sonner';
	import { api } from '$lib/api';
	import { i18n, t } from '$lib/i18n/index.svelte';
	import { isHosted } from '$lib/ipc.svelte';
	import { language, voice } from '$lib/voice.svelte';

	/** The language already spoken about; a different one is worth saying again. */
	let announced = $state<string | null>(null);
	/** The state at startup is not news — only a change the user just made is. */
	let started = false;

	const locale = $derived(i18n.intlLocale);
	const matching = $derived(voice.forLanguage(locale));

	const kind = $derived.by(() => {
		if (!voice.speaks) return null;
		if (voice.voices.length === 0) return null;
		if (matching.length === 0) return 'missing';
		// An explicit pick outlives the language it was made for; the automatic
		// choice already follows along and needs no notice.
		const chosen = voice.chosen;
		if (chosen && language(chosen.lang) !== language(locale)) return 'switch';
		return null;
	});

	$effect(() => {
		if (!started) {
			started = true;
			announced = locale;
			return;
		}

		if (!kind) {
			// Back in order, so the next change may speak up again.
			if (matching.length > 0) announced = null;
			return;
		}
		if (announced === locale) return;
		announced = locale;

		const label = new Intl.DisplayNames([locale], { type: 'language' }).of(locale) ?? locale;

		if (kind === 'missing') {
			toast.warning(t('voice.hint.missing.title', { language: label }), {
				description: t('voice.hint.missing.body'),
				duration: 12000,
				action: isHosted
					? {
							label: t('voice.hint.missing.action'),
							onClick: () => void api.openSpeechSettings()
						}
					: undefined
			});
			return;
		}

		const suggestion = matching[0];
		toast.info(t('voice.hint.switch.title', { language: label }), {
			description: t('voice.hint.switch.body', { voice: suggestion.name }),
			duration: 12000,
			action: {
				label: t('voice.hint.switch.action'),
				onClick: () => voice.setName(suggestion.name)
			}
		});
	});
</script>
