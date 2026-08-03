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
import { toast } from 'svelte-sonner';
import { cli } from '$lib/cli.svelte';
import { commandCatalog, commandFor, type Command } from '$lib/commands.svelte';
import { exact } from '$lib/format';
import { i18n, t } from '$lib/i18n/index.svelte';
import { isHosted } from '$lib/ipc.svelte';
import { costOf } from '$lib/pricing.svelte';

export const LOCAL_SOURCE = 'claude-code';

const KEY = 'claudeadmin.assistantSource';
const MODEL_KEY = 'claudeadmin.assistantModel';
const EFFORT_KEY = 'claudeadmin.assistantEffort';

const SHARE_KEY = 'claudeadmin.assistantShare';

/** Which parts of the summary a hosted model is allowed to see. */
export type Share = { models: boolean; tools: boolean; rhythm: boolean };

const SHARE_ALL: Share = { models: true, tools: true, rhythm: true };

export const SHARE_PARTS: (keyof Share)[] = ['models', 'tools', 'rhythm'];

function storedShare(): Share {
	if (typeof localStorage === 'undefined') return { ...SHARE_ALL };
	try {
		const raw: unknown = JSON.parse(localStorage.getItem(SHARE_KEY) ?? '{}');
		const value = raw as Partial<Share>;
		return {
			models: value.models ?? true,
			tools: value.tools ?? true,
			rhythm: value.rhythm ?? true
		};
	} catch {
		return { ...SHARE_ALL };
	}
}

function stored(key: string): string {
	return typeof localStorage === 'undefined' ? '' : (localStorage.getItem(key) ?? '');
}

export type Routed =
	| { kind: 'command'; command: Command }
	| { kind: 'question' }
	| { kind: 'clarify'; question: string };

/** The language to answer in, named in English because the prompt is. */
function languageName(): string {
	return new Intl.DisplayNames(['en'], { type: 'language' }).of(i18n.intlLocale) ?? 'English';
}

/**
 * The router's line, whatever it is wrapped in.
 *
 * Models fence JSON in markdown often enough that insisting on a bare line
 * would fail on a technicality; the first brace to the last is what counts.
 */
function parseRouted(
	text: string
): { kind: 'command'; id: string } | { kind: 'clarify'; question: string } | null {
	const start = text.indexOf('{');
	const end = text.lastIndexOf('}');
	if (start === -1 || end <= start) return null;

	try {
		const value: unknown = JSON.parse(text.slice(start, end + 1));
		if (typeof value !== 'object' || value === null) return null;
		const record = value as Record<string, unknown>;

		if (record.kind === 'command' && typeof record.id === 'string') {
			return { kind: 'command', id: record.id };
		}
		if (record.kind === 'clarify' && typeof record.question === 'string') {
			return { kind: 'clarify', question: record.question };
		}
	} catch {
		return null;
	}
	return null;
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

	/**
	 * What of the summary goes out with a question.
	 *
	 * Every part is on by default and can be switched off: the figures are the
	 * user's, and a hosted model is somebody else's machine.
	 */
	share = $state<Share>(storedShare());

	setShare(part: keyof Share, on: boolean): void {
		this.share = { ...this.share, [part]: on };
		localStorage.setItem(SHARE_KEY, JSON.stringify(this.share));
	}

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

		if (this.share.models && this.models.length > 0) {
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

		if (this.share.tools && this.tools.length > 0) {
			lines.push('Top tools (calls, sessions):');
			for (const row of this.tools.slice(0, 10)) {
				lines.push(`- ${row.name}: ${exact(row.calls)} calls in ${exact(row.sessions)} sessions`);
			}
			lines.push('');
		}

		if (this.share.rhythm && this.rhythm) {
			lines.push(
				`Active days: ${this.rhythm.activeDays}, longest streak ${this.rhythm.longestStreak}, ` +
					`current streak ${this.rhythm.currentStreak}.`
			);
		}

		return lines.join('\n');
	}

	/**
	 * What a spoken sentence turns out to be: a command, a question about the
	 * figures, or too vague to act on.
	 *
	 * The model only ever picks an id from `COMMANDS`; anything it invents is
	 * dropped and treated as a question, so a hallucinated action cannot run.
	 */
	async route(heard: string): Promise<Routed> {
		const prompt =
			'Decide what the user wants from a desktop app that shows Claude Code usage.\n' +
			'Answer with one line of JSON and nothing else:\n' +
			'{"kind":"command","id":"<id from the list>"} to carry a command out,\n' +
			'{"kind":"question"} if it is a question about the usage figures,\n' +
			`{"kind":"clarify","question":"<one short question, written in ${languageName()}>"} ` +
			'if it could be several of them.\n\nCommands:\n' +
			`${commandCatalog()}\n\nThe user said: ${JSON.stringify(heard)}`;

		let text: string;
		try {
			const answer = await api.askClaude(
				this.source,
				cli.configured,
				prompt,
				this.model || null,
				this.effort || null
			);
			text = answer.text;
		} catch {
			// A router that cannot be reached must not swallow the sentence: the
			// question path still has something to say about it.
			return { kind: 'question' };
		}

		const parsed = parseRouted(text);
		if (!parsed) return { kind: 'question' };
		if (parsed.kind === 'command') {
			const command = commandFor(parsed.id);
			return command ? { kind: 'command', command } : { kind: 'question' };
		}
		return parsed.kind === 'clarify' && parsed.question.trim() !== ''
			? { kind: 'clarify', question: parsed.question }
			: { kind: 'question' };
	}

	/**
	 * Acts on what was heard, and says what it did.
	 *
	 * Returns false when the sentence was a question after all, which is the
	 * caller's cue to run its normal ask-and-show flow.
	 */
	async obey(heard: string): Promise<{ handled: boolean; clarify?: string }> {
		const routed = await this.route(heard);

		if (routed.kind === 'command') {
			const name = t(routed.command.label);
			toast(t('voice.command.ask', { command: name }), {
				description: heard,
				duration: 15000,
				action: { label: t('voice.command.run'), onClick: () => routed.command.run() }
			});
			return { handled: true };
		}

		if (routed.kind === 'clarify') return { handled: true, clarify: routed.question };
		return { handled: false };
	}

	async ask(question: string): Promise<string> {
		// The window is translated, the prompt is not: without this the answer
		// comes back in English and a German voice reads English words.
		const language =
			new Intl.DisplayNames(['en'], { type: 'language' }).of(i18n.intlLocale) ?? 'English';

		const answer = await api.askClaude(
			this.source,
			cli.configured,
			`${this.context}\n\nQuestion: ${question.trim()}\n\n` +
				`Answer briefly, using only the figures above. Write the answer in ${language}.`,
			this.model || null,
			this.effort || null
		);
		this.last = answer;
		return answer.text;
	}
}

export const assistant = new Assistant();
