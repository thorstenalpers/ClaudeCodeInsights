/**
 * Talking to the assistant, and being talked back to.
 *
 * The microphone only listens when its button is pressed: dictation that
 * starts itself is a microphone nobody asked for. Recognition runs through
 * Windows' own on-device recogniser in the host, not through a web speech
 * service, so nothing said here leaves the machine.
 *
 * Reading back has two engines. The WebView's own synthesis uses the voices
 * Windows has installed, which on most machines is English and nothing else.
 * A downloaded Kokoro pack fills that gap and runs in the host. Both are
 * chosen in the same place, because to a reader they are one setting.
 */
import { toast } from 'svelte-sonner';
import { api, type HubVoice, type VoicePack } from '$lib/api';
import { i18n, t } from '$lib/i18n/index.svelte';
import { isHosted } from '$lib/ipc.svelte';
import { logs } from '$lib/logs.svelte';

const SPEAK_KEY = 'claudeadmin.voiceOutput';
const VOICE_KEY = 'claudeadmin.voiceName';

/** A pack speaker is stored as `pack:<id>:<speaker>` in the same setting the
 *  Windows voices use, so one choice covers both kinds. */
export function packChoice(id: string, speaker: number): string {
	return `pack:${id}:${speaker}`;
}

function parsePack(value: string): { id: string; speaker: number } | null {
	// Split at the last colon rather than at every one: a pack from the hub
	// carries its repository in the id, colon and slash and all.
	if (!value.startsWith('pack:')) return null;
	const at = value.lastIndexOf(':');
	if (at < 'pack:'.length) return null;
	return { id: value.slice('pack:'.length, at), speaker: Number(value.slice(at + 1)) || 0 };
}

/** `de`, `de-DE` and `de_DE` all mean the same thing to a reader. */
export function language(tag: string): string {
	return tag.toLowerCase().replace('_', '-').split('-')[0];
}

class Voice {
	speaks = $state<boolean>(
		typeof localStorage === 'undefined' ? false : localStorage.getItem(SPEAK_KEY) === 'on'
	);
	listening = $state(false);
	/** True while something is being read out, so it can be stopped again. */
	speaking = $state(false);
	error = $state<string | null>(null);
	/** null until the host has been asked; false when Windows has no recogniser. */
	available = $state<boolean | null>(null);
	/** Windows will not listen until its speech privacy setting is on. */
	needsPrivacy = $state(false);

	/** The installed Windows voices, once the engine has listed them. */
	voices = $state<SpeechSynthesisVoice[]>([]);
	/** The downloadable packs and whether each is here. */
	packs = $state<VoicePack[]>([]);
	/** Where packs are unpacked, shown so the folder is no mystery. */
	folder = $state<string>('');
	/** Which pack is being fetched, so the button can say so. */
	installing = $state<string | null>(null);
	/** Bytes in so far, and what the server promised, for the progress bar. */
	received = $state(0);
	total = $state<number | null>(null);
	/** Empty means: whichever voice speaks the app's language. */
	name = $state<string>(
		typeof localStorage === 'undefined' ? '' : (localStorage.getItem(VOICE_KEY) ?? '')
	);

	// The list is empty on the first call and arrives with the event, so both
	// have to be taken.
	init(): void {
		if (typeof speechSynthesis === 'undefined') return;
		this.voices = speechSynthesis.getVoices();
		speechSynthesis.addEventListener('voiceschanged', () => {
			this.voices = speechSynthesis.getVoices();
		});
	}

	/** Re-reads the folder: a pack put there by hand counts as installed. */
	async loadPacks(): Promise<void> {
		if (!isHosted) return;
		this.packs = await api.listVoicePacks().catch(() => []);
		this.folder = await api.voicePacksFolder().catch(() => '');
	}

	/** What the hub was asked for, and what it answered. */
	hubResults = $state<HubVoice[]>([]);
	hubSearching = $state(false);
	/** Empty until a search has run; a note when it found nothing usable. */
	hubNote = $state<string | null>(null);

	/** Asks Hugging Face for voices by name, e.g. "kokoro german". */
	async searchHub(query: string): Promise<void> {
		if (!isHosted || !query.trim()) return;
		this.hubSearching = true;
		this.hubNote = null;
		logs.info('voice', `searching the hub for ${query}`);
		try {
			this.hubResults = await api.searchVoiceHub(query);
			if (this.hubResults.length === 0) this.hubNote = t('settings.voice.hub.none');
		} catch (cause) {
			this.hubResults = [];
			this.hubNote = cause instanceof Error ? cause.message : String(cause);
		} finally {
			this.hubSearching = false;
		}
	}

	/**
	 * Fetches a pack; minutes of download, so the caller shows the state.
	 *
	 * `repo` is set when the pack comes from the hub rather than from the list
	 * this app ships, which is the only difference the download makes.
	 */
	async install(id: string, repo?: string, megabytes?: number): Promise<void> {
		if (!isHosted || this.installing) return;
		const pack = this.packs.find((entry) => entry.id === id) ?? { megabytes: megabytes ?? 0 };
		this.installing = id;
		this.received = 0;
		this.total = null;
		this.error = null;
		logs.info('voice', `installing pack ${id}`);

		const { listen } = await import('@tauri-apps/api/event');
		const stop = await listen<{ id: string; received: number; total: number | null }>(
			'voice:progress',
			(event) => {
				if (event.payload.id !== id) return;
				this.received = event.payload.received;
				this.total = event.payload.total;
			}
		);

		// Minutes of download with no other sign of life, so the state is said
		// out loud rather than left to a disabled button.
		const notice = toast.loading(t('settings.voice.packs.installing'), {
			description: t('settings.voice.packs.size', { size: pack?.megabytes ?? 0 }),
			duration: Number.POSITIVE_INFINITY,
			action: { label: t('settings.voice.packs.cancel'), onClick: () => void this.cancel() }
		});

		try {
			const installed = repo ? await api.installHubVoice(repo) : await api.installVoicePack(id);
			await this.loadPacks();
			logs.info('voice', `pack ${id} installed`);
			toast.success(t('settings.voice.packs.done', { label: installed.label }), {
				id: notice,
				description: this.folder,
				duration: 8000
			});
		} catch (cause) {
			this.error = cause instanceof Error ? cause.message : String(cause);
			// A cancel is the user's own doing, not a failure to report as one.
			if (this.error.includes('cancelled')) {
				logs.info('voice', `pack ${id} cancelled`);
				toast.info(t('settings.voice.packs.cancelled'), { id: notice, duration: 5000 });
				this.error = null;
			} else {
				logs.error('voice', `pack ${id} failed: ${this.error}`);
				toast.error(t('settings.voice.packs.failed'), {
					id: notice,
					description: this.error,
					duration: 12000
				});
			}
		} finally {
			stop();
			this.installing = null;
			this.received = 0;
			this.total = null;
		}
	}

	/** Asks the host to stop the running download. */
	async cancel(): Promise<void> {
		if (!isHosted || !this.installing) return;
		await api.cancelVoicePack().catch(() => {});
	}

	/** How far the running download has come, 0–100, or null while unknown. */
	get percent(): number | null {
		if (!this.total) return null;
		return Math.min(100, Math.round((this.received / this.total) * 100));
	}

	async remove(id: string): Promise<void> {
		if (!isHosted) return;
		await api.removeVoicePack(id).catch(() => {});
		if (parsePack(this.name)?.id === id) this.setName('');
		await this.loadPacks();
	}

	/** The voices that speak a language, best match first. */
	forLanguage(tag: string): SpeechSynthesisVoice[] {
		const wanted = tag.toLowerCase().replace('_', '-');
		return this.voices
			.filter((voice) => language(voice.lang) === language(wanted))
			.sort(
				(a, b) => Number(b.lang.toLowerCase() === wanted) - Number(a.lang.toLowerCase() === wanted)
			);
	}

	/** The voice actually set, as opposed to the one the language implies. */
	get chosen(): SpeechSynthesisVoice | null {
		return this.voices.find((entry) => entry.name === this.name) ?? null;
	}

	setName(next: string): void {
		this.name = next;
		if (next === '') localStorage.removeItem(VOICE_KEY);
		else localStorage.setItem(VOICE_KEY, next);
	}

	setSpeaks(next: boolean): void {
		this.speaks = next;
		localStorage.setItem(SPEAK_KEY, next ? 'on' : 'off');
		if (!next) this.silence();
	}

	async check(): Promise<void> {
		if (!isHosted) {
			this.available = false;
			return;
		}
		this.available = await api.speechAvailable().catch(() => false);
	}

	/** Resolves with what was heard, or null when nothing was. */
	async listen(): Promise<string | null> {
		if (!isHosted || this.listening) return null;
		this.listening = true;
		this.error = null;
		try {
			const heard = await api.recognizeSpeech(i18n.intlLocale);
			return heard.trim() === '' ? null : heard;
		} catch (cause) {
			// The host marks the one refusal the user can undo; everything else
			// is passed through as Windows worded it.
			const message = cause instanceof Error ? cause.message : String(cause);
			this.needsPrivacy = message.includes('speech-privacy-not-accepted');
			this.error = this.needsPrivacy ? null : message;
			return null;
		} finally {
			this.listening = false;
		}
	}

	speak(text: string): void {
		this.silence();

		// A downloaded pack speaks in the host, where the model lives; the
		// WebView never sees it.
		const pack = parsePack(this.name);
		if (pack) {
			this.speaking = true;
			void api
				.speakText(pack.id, pack.speaker, text)
				.catch((cause: unknown) => {
					this.error = cause instanceof Error ? cause.message : String(cause);
				})
				.finally(() => (this.speaking = false));
			return;
		}

		if (typeof speechSynthesis === 'undefined') return;
		const utterance = new SpeechSynthesisUtterance(text);
		utterance.lang = i18n.intlLocale;
		// `lang` alone is a wish, not an instruction: with no voice set the engine
		// reads German through whatever voice Windows starts with, which is the
		// English one on most machines.
		const voice =
			this.voices.find((entry) => entry.name === this.name) ?? this.forLanguage(i18n.intlLocale)[0];
		if (voice) utterance.voice = voice;

		// The engine's own events, not a guess: a long answer ends when it ends,
		// and the stop button has to disappear exactly then.
		utterance.onend = () => (this.speaking = false);
		utterance.onerror = () => (this.speaking = false);
		this.speaking = true;
		speechSynthesis.speak(utterance);
	}

	silence(): void {
		if (typeof speechSynthesis !== 'undefined') speechSynthesis.cancel();
		// The pack speaks in the host, where cancelling the WebView's synthesis
		// reaches nothing: the sentence has to be cut where it is being played.
		if (isHosted) void api.stopSpeaking().catch(() => {});
		this.speaking = false;
	}
}

export const voice = new Voice();
