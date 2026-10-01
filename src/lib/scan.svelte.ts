import { listen } from '@tauri-apps/api/event';
import { api, type ScanProgress, type ScanStats } from './api';
import { isHosted } from './ipc.svelte';

type Status = 'idle' | 'running' | 'done' | 'failed';

/**
 * The scan's state, fed by the events the Rust side pushes.
 *
 * It lives outside any page so a scan started on Overview keeps reporting while
 * the user is somewhere else.
 */
class Scan {
	status = $state<Status>('idle');
	filesDone = $state(0);
	filesTotal = $state(0);
	currentFile = $state<string | null>(null);
	stats = $state<ScanStats | null>(null);
	error = $state<string | null>(null);
	roots = $state<string[]>([]);

	/** Bumped whenever new data has landed, so views know to refetch. */
	dataVersion = $state(0);

	get percent(): number {
		return this.filesTotal > 0 ? Math.round((this.filesDone / this.filesTotal) * 100) : 0;
	}

	async init(): Promise<void> {
		if (!isHosted) return;

		await listen<ScanProgress>('scan:progress', (event) => {
			this.status = 'running';
			this.filesDone = event.payload.filesDone;
			this.filesTotal = event.payload.filesTotal;
			this.currentFile = event.payload.currentFile;
		});

		await listen<ScanStats>('scan:completed', (event) => {
			this.status = 'done';
			this.stats = event.payload;
			this.currentFile = null;
			this.error = event.payload.filesFailed > 0 ? event.payload.firstError : null;
			this.dataVersion += 1;
		});

		await listen<string>('scan:failed', (event) => {
			this.status = 'failed';
			this.error = event.payload;
			this.currentFile = null;
		});

		const state = await api.getScanState();
		this.roots = state.roots;
		this.status = state.running ? 'running' : 'idle';
	}

	async start(): Promise<void> {
		if (!isHosted || this.status === 'running') return;
		this.error = null;
		this.status = 'running';
		this.filesDone = 0;
		this.filesTotal = 0;
		await api.startScan();
	}
}

export const scan = new Scan();
