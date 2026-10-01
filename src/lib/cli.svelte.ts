/**
 * Where the Claude Code binary lives, when the user has told us.
 *
 * Empty means "look in the usual places", which is what the host does. The path
 * is a setting rather than a guess so an unusual install is a text field away
 * instead of a support question.
 */
import { api, type CliStatus } from '$lib/api';
import { isHosted } from '$lib/ipc.svelte';
import { logs } from '$lib/logs.svelte';

const KEY = 'claudeadmin.cliPath';

class Cli {
	path = $state<string>(
		typeof localStorage === 'undefined' ? '' : (localStorage.getItem(KEY) ?? '')
	);

	/** null rather than '' so the host reads it as "not configured". */
	get configured(): string | null {
		return this.path.trim() === '' ? null : this.path.trim();
	}

	set(next: string): void {
		this.path = next;
		if (next.trim() === '') {
			localStorage.removeItem(KEY);
		} else {
			localStorage.setItem(KEY, next.trim());
		}
		void this.refresh();
	}

	/** What the host found, once it has been asked. */
	status = $state<CliStatus | null>(null);

	/** Asks the host again — after a path change, or once at startup. */
	async refresh(): Promise<void> {
		if (!isHosted) return;
		this.status = await api.getCliStatus(this.configured).catch(() => null);
		logs.info('cli', this.status?.found ? `found ${this.status.version ?? '?'}` : 'not found');
	}
}

export const cli = new Cli();
