/**
 * What a spoken sentence is allowed to set in motion.
 *
 * A closed list rather than a free hand: the model picks an id from here, so
 * the worst a misheard sentence can do is open the wrong page. Nothing that
 * deletes, writes to `~/.claude.json`, or spends money is in here, and the
 * automatic mode in the settings does not widen it.
 *
 * The intents are English on purpose. The window is translated, the prompt is
 * not, and taking them from the English catalog keeps one wording for both.
 */
import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { en, type MessageKey } from '$lib/i18n/en';
import { PAGES, SETTINGS_PAGE } from '$lib/pages';
import { scan } from '$lib/scan.svelte';
import { theme } from '$lib/theme.svelte';

export type Command = {
	id: string;
	/** How the command is described to the model. */
	intent: string;
	/** How the window names it afterwards. */
	label: MessageKey;
	run: () => void;
};

const navigation: Command[] = [...PAGES, SETTINGS_PAGE].map((page) => ({
	id: `open:${page.href}`,
	intent: `Open the ${en[page.label]} page — ${en[page.description]}`,
	label: page.label,
	run: () => void goto(resolve(page.href))
}));

const actions: Command[] = [
	{
		id: 'scan',
		intent: 'Rescan the transcripts on this machine for new sessions',
		label: 'header.rescan',
		run: () => void scan.start()
	},
	{
		id: 'theme:dark',
		intent: 'Switch the window to dark mode',
		label: 'settings.mode.dark',
		run: () => theme.setMode('dark')
	},
	{
		id: 'theme:light',
		intent: 'Switch the window to light mode',
		label: 'settings.mode.light',
		run: () => theme.setMode('light')
	},
	{
		id: 'theme:system',
		intent: 'Let the operating system decide light or dark',
		label: 'settings.mode.system',
		run: () => theme.setMode('system')
	}
];

export const COMMANDS: readonly Command[] = [...navigation, ...actions];

export function commandFor(id: string): Command | null {
	return COMMANDS.find((command) => command.id === id) ?? null;
}

/** The list as the prompt sees it: one id per line, nothing else. */
export function commandCatalog(): string {
	return COMMANDS.map((command) => `- ${command.id}: ${command.intent}`).join('\n');
}
