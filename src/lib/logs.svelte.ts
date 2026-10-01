/**
 * What the two halves of the app have been doing, kept for the log view.
 *
 * Two buffers rather than one merged stream: the host and the window fail in
 * different ways, and a reader looking for a download that stalled wants the
 * host's account of it without the window's rendering chatter in between.
 *
 * Both are ring buffers. A log that grows without a bound is a memory leak
 * with a friendly name.
 */
import { toast } from 'svelte-sonner';
import { t } from '$lib/i18n/index.svelte';
import { isHosted } from '$lib/ipc.svelte';

export type LogLevel = 'error' | 'warn' | 'info' | 'debug';

export type LogLine = {
	at: number;
	level: LogLevel;
	/** Where it came from — a module path in the host, a store in the window. */
	source: string;
	message: string;
};

const LIMIT = 2000;
const ENABLED_KEY = 'claudeadmin.logView';

/** Tauri's log plugin numbers its levels; these are the names they carry. */
const LEVELS: Record<number, LogLevel> = {
	1: 'debug',
	2: 'debug',
	3: 'info',
	4: 'warn',
	5: 'error'
};

/** `[2026-08-03][12:00:00][claude_admin::tts][INFO] fetching …` */
const HOST_LINE = /^\[[^\]]*\]\[[^\]]*\]\[([^\]]*)\]\[[^\]]*\]\s?([\s\S]*)$/;

class Logs {
	/** Lines from Rust, forwarded by the log plugin's webview target. */
	host = $state<LogLine[]>([]);
	/** Lines this window wrote about its own work. */
	app = $state<LogLine[]>([]);
	/** The view is off by default: it is a tool, not part of the daily surface. */
	enabled = $state<boolean>(
		typeof localStorage === 'undefined' ? false : localStorage.getItem(ENABLED_KEY) === 'on'
	);

	setEnabled(next: boolean): void {
		this.enabled = next;
		localStorage.setItem(ENABLED_KEY, next ? 'on' : 'off');
	}

	/** Records a line from the window itself. */
	write(level: LogLevel, source: string, message: string): void {
		push(this.app, { at: Date.now(), level, source, message });
	}

	info = (source: string, message: string) => this.write('info', source, message);
	warn = (source: string, message: string) => this.write('warn', source, message);
	error = (source: string, message: string) => this.write('error', source, message);

	clear(which: 'host' | 'app'): void {
		this[which] = [];
	}

	/** Starts taking the host's lines. Safe to call in the browser: it does nothing. */
	async listen(): Promise<void> {
		if (!isHosted) return;
		const { listen } = await import('@tauri-apps/api/event');
		await listen<{ level: number; message: string }>('log://log', (event) => {
			const match = HOST_LINE.exec(event.payload.message);
			const line: LogLine = {
				at: Date.now(),
				level: LEVELS[event.payload.level] ?? 'info',
				source: match?.[1] ?? 'host',
				message: (match?.[2] ?? event.payload.message).trim()
			};
			push(this.host, line);

			// Nobody watches the log view while working, so a failure in the host
			// would otherwise pass in silence. The id makes a line that repeats
			// replace its own toast instead of stacking a column of them.
			if (line.level === 'error') {
				toast.error(t('logs.toast.error'), {
					id: `log:${line.source}:${line.message}`,
					description: `${line.source}: ${line.message}`,
					duration: 12000
				});
			}
		});
	}
}

function push(into: LogLine[], line: LogLine): void {
	into.push(line);
	if (into.length > LIMIT) into.splice(0, into.length - LIMIT);
}

export const logs = new Logs();
