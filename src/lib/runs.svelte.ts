/**
 * The coding sessions this app is running, and what they are waiting for.
 *
 * Everything here arrives as events from the host: lines of the conversation,
 * changes of state, and the tool calls that stop and ask. A run holds its own
 * lines so switching between two of them does not lose either.
 */
import { api, type Ask, type RunInfo, type RunLine, type RunRequest } from '$lib/api';
import { isHosted } from '$lib/ipc.svelte';
import { logs } from '$lib/logs.svelte';

/** Old lines are dropped: a console is a window, not the transcript. */
const KEEP = 400;

class Runs {
	running = $state<RunInfo[]>([]);
	/** Lines per run id, oldest first. */
	lines = $state<Record<string, RunLine[]>>({});
	/** What each run is doing: `running`, `idle`, `ended`, `failed`. */
	state = $state<Record<string, string>>({});
	/** Tools waiting for an answer, oldest first. */
	asks = $state<Ask[]>([]);
	error = $state<string | null>(null);
	/** Which run the console shows; empty means the first one. */
	open = $state<string>('');

	private listening = false;

	/** Starts taking the host's events. Safe to call more than once. */
	async listen(): Promise<void> {
		if (!isHosted || this.listening) return;
		this.listening = true;

		const { listen } = await import('@tauri-apps/api/event');

		await listen<RunLine>('session:message', (event) => {
			const line = event.payload;
			const kept = [...(this.lines[line.run] ?? []), line].slice(-KEEP);
			this.lines = { ...this.lines, [line.run]: kept };
		});

		await listen<{ id: string; state: string; message?: string }>('session:state', (event) => {
			const { id, state, message } = event.payload;
			this.state = { ...this.state, [id]: state };
			if (state === 'failed' && message) {
				this.error = message;
				logs.error('runs', `${id}: ${message}`);
			}
			if (state === 'ended') {
				this.running = this.running.filter((run) => run.id !== id);
				// A run that ends with a question still open would leave the
				// dialog on screen with nothing behind it.
				this.asks = this.asks.filter((ask) => ask.run !== id);
			}
		});

		await listen<Ask>('session:permission', (event) => {
			this.asks = [...this.asks, event.payload];
		});

		this.running = await api.listRunningSessions().catch(() => []);
	}

	async start(request: RunRequest): Promise<void> {
		if (!isHosted) return;
		this.error = null;
		try {
			const info = await api.startSession(request);
			this.running = [...this.running, info];
			this.open = info.id;
			logs.info('runs', `started ${info.id} in ${info.project}`);
		} catch (cause) {
			this.error = cause instanceof Error ? cause.message : String(cause);
		}
	}

	async send(id: string, prompt: string): Promise<void> {
		if (prompt.trim() === '') return;
		// Shown at once: the answer takes seconds and a console that swallows
		// what was typed looks broken.
		const line: RunLine = { run: id, kind: 'you', text: prompt, tool: null, costUsd: null };
		this.lines = { ...this.lines, [id]: [...(this.lines[id] ?? []), line].slice(-KEEP) };
		await api.sendToSession(id, prompt).catch((cause: unknown) => {
			this.error = cause instanceof Error ? cause.message : String(cause);
		});
	}

	async interrupt(id: string): Promise<void> {
		await api.interruptSession(id).catch(() => {});
	}

	async stop(id: string): Promise<void> {
		await api.stopSession(id).catch(() => {});
	}

	/** Answers the oldest waiting tool of a run, or a named one. */
	async answer(askId: string, allow: boolean): Promise<void> {
		this.asks = this.asks.filter((ask) => ask.id !== askId);
		await api.answerPermission(askId, allow).catch(() => {});
	}

	linesOf(id: string): RunLine[] {
		return this.lines[id] ?? [];
	}
}

export const runs = new Runs();
