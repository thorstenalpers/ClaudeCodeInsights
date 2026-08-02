/**
 * Which source answers a question.
 *
 * The local Claude Code binary is the default because it needs no key and
 * sends nothing from this app. A hosted provider is an explicit choice, and it
 * is the only thing in this window that puts data on the network.
 */
export const LOCAL_SOURCE = 'claude-code';

const KEY = 'claudeadmin.assistantSource';

class Assistant {
	source = $state<string>(
		typeof localStorage === 'undefined' ? LOCAL_SOURCE : (localStorage.getItem(KEY) ?? LOCAL_SOURCE)
	);

	set(next: string): void {
		this.source = next;
		localStorage.setItem(KEY, next);
	}
}

export const assistant = new Assistant();
