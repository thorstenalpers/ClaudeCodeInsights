/**
 * Talking to the assistant, and being talked back to.
 *
 * Input stays on the keyboard until it is switched over: dictation that starts
 * itself is a microphone nobody asked for. Recognition runs through Windows'
 * own on-device recogniser in the host, not through a web speech service, so
 * nothing said here leaves the machine. Output uses the WebView's speech
 * synthesis, which reads the Windows voices already installed.
 */
import { api } from '$lib/api';
import { i18n } from '$lib/i18n/index.svelte';
import { isHosted } from '$lib/ipc.svelte';

export type InputMode = 'manual' | 'speech';

const MODE_KEY = 'claudeadmin.voiceInput';
const SPEAK_KEY = 'claudeadmin.voiceOutput';

function readMode(): InputMode {
	if (typeof localStorage === 'undefined') return 'manual';
	return localStorage.getItem(MODE_KEY) === 'speech' ? 'speech' : 'manual';
}

class Voice {
	mode = $state<InputMode>(readMode());
	speaks = $state<boolean>(
		typeof localStorage === 'undefined' ? false : localStorage.getItem(SPEAK_KEY) === 'on'
	);
	listening = $state(false);
	error = $state<string | null>(null);
	/** null until the host has been asked; false when Windows has no recogniser. */
	available = $state<boolean | null>(null);

	setMode(next: InputMode): void {
		this.mode = next;
		localStorage.setItem(MODE_KEY, next);
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
			this.error = cause instanceof Error ? cause.message : String(cause);
			return null;
		} finally {
			this.listening = false;
		}
	}

	speak(text: string): void {
		if (typeof speechSynthesis === 'undefined') return;
		this.silence();
		const utterance = new SpeechSynthesisUtterance(text);
		utterance.lang = i18n.intlLocale;
		speechSynthesis.speak(utterance);
	}

	silence(): void {
		if (typeof speechSynthesis !== 'undefined') speechSynthesis.cancel();
	}
}

export const voice = new Voice();
