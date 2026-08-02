/**
 * Which source answers a question, and the summary it is given to answer from.
 *
 * The local Claude Code binary is the default because it needs no key and
 * sends nothing from this app. A hosted provider is an explicit choice, and it
 * is the only thing in this window that puts data on the network.
 *
 * The figures live here rather than in a view because two places ask the same
 * question now — the page and the slide-over — and a second copy of the prompt
 * would be a second thing to keep true.
 */
import { api, type Answer, type ModelRow, type Rhythm, type ToolRow } from '$lib/api';
import { cli } from '$lib/cli.svelte';
import { exact } from '$lib/format';
import { isHosted } from '$lib/ipc.svelte';
import { costOf } from '$lib/pricing.svelte';

export const LOCAL_SOURCE = 'claude-code';

const KEY = 'claudeadmin.assistantSource';
const MODEL_KEY = 'claudeadmin.assistantModel';
const EFFORT_KEY = 'claudeadmin.assistantEffort';

function stored(key: string): string {
	return typeof localStorage === 'undefined' ? '' : (localStorage.getItem(key) ?? '');
}

class Assistant {
	source = $state<string>(
		typeof localStorage === 'undefined' ? LOCAL_SOURCE : (localStorage.getItem(KEY) ?? LOCAL_SOURCE)
	);

	/** Empty means: leave the CLI on whatever it is configured for. */
	model = $state<string>(stored(MODEL_KEY));
	effort = $state<string>(stored(EFFORT_KEY));
	/** What the last answer says actually served it, which can differ. */
	last = $state<Answer | null>(null);

	models = $state<ModelRow[]>([]);
	tools = $state<ToolRow[]>([]);
	rhythm = $state<Rhythm | null>(null);

	set(next: string): void {
		this.source = next;
		localStorage.setItem(KEY, next);
	}

	setModel(next: string): void {
		this.model = next;
		if (next === '') localStorage.removeItem(MODEL_KEY);
		else localStorage.setItem(MODEL_KEY, next);
	}

	setEffort(next: string): void {
		this.effort = next;
		if (next === '') localStorage.removeItem(EFFORT_KEY);
		else localStorage.setItem(EFFORT_KEY, next);
	}

	/** Refreshed on a scan; cheap enough that both callers can ask for it. */
	async load(): Promise<void> {
		if (!isHosted) return;
		const [models, tools, rhythm] = await Promise.all([
			api.listModels(),
			api.listTools(),
			api.getRhythm()
		]);
		this.models = models;
		this.tools = tools;
		this.rhythm = rhythm;
	}

	/**
	 * The figures, as plain text.
	 *
	 * Deliberately a summary rather than the database: the question is about
	 * totals, and a transcript would be both useless here and the one thing this
	 * app promises never to hand around.
	 */
	get context(): string {
		const lines: string[] = ['Claude Code usage on this machine.', ''];

		if (this.models.length > 0) {
			lines.push('Models (turns, input, output, cache read, cost at API rates in USD):');
			for (const row of this.models) {
				lines.push(
					`- ${row.model}: ${exact(row.turns)} turns, ${exact(row.inputTokens)} in, ` +
						`${exact(row.outputTokens)} out, ${exact(row.cacheReadTokens)} cache read, ` +
						`$${costOf(row.model, row).toFixed(2)}`
				);
			}
			lines.push('');
		}

		if (this.tools.length > 0) {
			lines.push('Top tools (calls, sessions):');
			for (const row of this.tools.slice(0, 10)) {
				lines.push(`- ${row.name}: ${exact(row.calls)} calls in ${exact(row.sessions)} sessions`);
			}
			lines.push('');
		}

		if (this.rhythm) {
			lines.push(
				`Active days: ${this.rhythm.activeDays}, longest streak ${this.rhythm.longestStreak}, ` +
					`current streak ${this.rhythm.currentStreak}.`
			);
		}

		return lines.join('\n');
	}

	async ask(question: string): Promise<string> {
		const answer = await api.askClaude(
			this.source,
			cli.configured,
			`${this.context}\n\nQuestion: ${question.trim()}\n\nAnswer briefly, using only the figures above.`,
			this.model || null,
			this.effort || null
		);
		this.last = answer;
		return answer.text;
	}
}

export const assistant = new Assistant();
