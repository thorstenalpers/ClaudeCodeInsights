/**
 * The single channel to the Rust backend.
 *
 * Tauri already supplies the envelope, correlation and error propagation a
 * hand-rolled bridge would need, so this module adds only what it does not: a
 * check for whether a host is attached at all, so the UI can run in a plain
 * browser under `npm run dev`, and a rejection that is an Error rather than the
 * `{ kind, message }` object the host sends.
 */
import { invoke as tauriInvoke } from '@tauri-apps/api/core';

/** False under `npm run dev` in a browser tab, true inside the app window. */
export const isHosted: boolean = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export type ErrorKind = 'database' | 'io' | 'internal' | 'badRequest' | 'unknown';

export class HostError extends Error {
	constructor(
		message: string,
		readonly kind: ErrorKind
	) {
		super(message);
		this.name = 'HostError';
	}
}

function isWireError(value: unknown): value is { kind: ErrorKind; message: string } {
	return (
		typeof value === 'object' &&
		value !== null &&
		typeof (value as { message?: unknown }).message === 'string' &&
		typeof (value as { kind?: unknown }).kind === 'string'
	);
}

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	if (!isHosted) {
		throw new HostError(`No host attached; '${command}' cannot be invoked.`, 'unknown');
	}

	try {
		return await tauriInvoke<T>(command, args);
	} catch (cause) {
		// Commands reject with the serialised Error enum; anything else came from
		// the bridge itself and only ever has a string to offer.
		throw isWireError(cause)
			? new HostError(cause.message, cause.kind)
			: new HostError(String(cause), 'unknown');
	}
}

/** The message to show a user, whatever was thrown. */
export function errorMessage(cause: unknown): string {
	return cause instanceof Error ? cause.message : String(cause);
}

/** Tells the host that this half of the startup is done. */
export async function reportReady(task: 'frontend' | 'backend'): Promise<void> {
	if (!isHosted) return;
	await invoke('set_complete', { task });
}
