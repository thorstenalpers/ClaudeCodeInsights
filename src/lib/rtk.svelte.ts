/**
 * RTK, the proxy that shortens command output before a model ever sees it.
 *
 * Two settings and nothing more: where its binary is, and whether this app
 * looks at it at all. Switched off, the page leaves the rail and the host is
 * never asked — RTK itself keeps filtering, which is not this app's business.
 */
import { api, type RtkStatus } from '$lib/api';
import { isHosted } from '$lib/ipc.svelte';
import { logs } from '$lib/logs.svelte';

const PATH_KEY = 'claudeadmin.rtkPath';
const ENABLED_KEY = 'claudeadmin.rtk';

class Rtk {
	path = $state<string>(
		typeof localStorage === 'undefined' ? '' : (localStorage.getItem(PATH_KEY) ?? '')
	);
	/** On by default; the rail entry still waits until there is something to show. */
	enabled = $state<boolean>(
		typeof localStorage === 'undefined' ? true : localStorage.getItem(ENABLED_KEY) !== 'off'
	);
	status = $state<RtkStatus | null>(null);

	/** null rather than '' so the host reads it as "look in the usual places". */
	get configured(): string | null {
		return this.path.trim() === '' ? null : this.path.trim();
	}

	/**
	 * Whether the rail carries the page.
	 *
	 * A history without the binary still counts: the numbers are worth reading
	 * on a machine where RTK was uninstalled but its record survived.
	 */
	get available(): boolean {
		return this.enabled && (this.status?.found === true || this.status?.historyExists === true);
	}

	setPath(next: string): void {
		this.path = next;
		if (next.trim() === '') {
			localStorage.removeItem(PATH_KEY);
		} else {
			localStorage.setItem(PATH_KEY, next.trim());
		}
		void this.refresh();
	}

	setEnabled(next: boolean): void {
		this.enabled = next;
		localStorage.setItem(ENABLED_KEY, next ? 'on' : 'off');
		if (next) void this.refresh();
		else this.status = null;
	}

	async refresh(): Promise<void> {
		if (!isHosted || !this.enabled) return;
		this.status = await api.getRtkStatus(this.configured).catch(() => null);
		logs.info('rtk', this.status?.found ? `found ${this.status.version ?? '?'}` : 'not found');
	}
}

export const rtk = new Rtk();
