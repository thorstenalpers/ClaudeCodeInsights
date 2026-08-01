/**
 * The single channel to the Rust backend.
 *
 * Tauri already supplies the envelope, correlation and error propagation a
 * hand-rolled bridge would need, so this module adds only what it does not: a
 * check for whether a host is attached at all, so the UI can run in a plain
 * browser under `npm run dev` and Storybook.
 */
import { invoke as tauriInvoke } from '@tauri-apps/api/core';

/** False under `npm run dev` in a browser tab, true inside the app window. */
export const isHosted: boolean = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	if (!isHosted) {
		throw new Error(`No host attached; '${command}' cannot be invoked.`);
	}
	return tauriInvoke<T>(command, args);
}

/** Tells the host that this half of the startup is done. */
export async function reportReady(task: 'frontend' | 'backend'): Promise<void> {
	if (!isHosted) return;
	await invoke('set_complete', { task });
}
