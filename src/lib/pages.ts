import type { Component } from 'svelte';
import type { Pathname } from '$app/types';
import type { MessageKey } from '$lib/i18n/en';
import Activity from '@lucide/svelte/icons/calendar-clock';
import Agents from '@lucide/svelte/icons/bot';
import Assistant from '@lucide/svelte/icons/sparkles';
import Cost from '@lucide/svelte/icons/circle-dollar-sign';
import Overview from '@lucide/svelte/icons/layout-dashboard';
import Projects from '@lucide/svelte/icons/folder-git-2';
import Sessions from '@lucide/svelte/icons/messages-square';
import Settings from '@lucide/svelte/icons/settings';
import Logs from '@lucide/svelte/icons/scroll-text';
import Tools from '@lucide/svelte/icons/wrench';
import Info from '@lucide/svelte/icons/info';
import Orchestration from '@lucide/svelte/icons/list-checks';

export type PageDefinition = {
	href: Pathname;
	/** Resolved at render time so a language change re-labels the rail. */
	label: MessageKey;
	/** Shown under the page title; says what the page answers. */
	description: MessageKey;
	icon: Component;
};

/** The pages reachable from the rail, in order. Settings is pinned separately. */
export const PAGES: readonly PageDefinition[] = [
	{
		href: '/',
		label: 'nav.overview',
		description: 'nav.overview.description',
		icon: Overview
	},
	{
		href: '/sessions',
		label: 'nav.sessions',
		description: 'nav.sessions.description',
		icon: Sessions
	},
	{
		href: '/cost',
		label: 'nav.cost',
		description: 'nav.cost.description',
		icon: Cost
	},
	{
		href: '/projects',
		label: 'nav.projects',
		description: 'nav.projects.description',
		icon: Projects
	},
	{
		href: '/activity',
		label: 'nav.activity',
		description: 'nav.activity.description',
		icon: Activity
	},
	{
		href: '/agents',
		label: 'nav.agents',
		description: 'nav.agents.description',
		icon: Agents
	},
	{
		href: '/tools',
		label: 'nav.tools',
		description: 'nav.tools.description',
		icon: Tools
	},
	{
		href: '/orchestration',
		label: 'nav.orchestration',
		description: 'nav.orchestration.description',
		icon: Orchestration
	},
	{
		href: '/assistant',
		label: 'nav.assistant',
		description: 'nav.assistant.description',
		icon: Assistant
	}
];

/**
 * One entry hung under a rail entry: a project, an activity, a model.
 *
 * A project carries the sessions that ran in it, which is the one place the
 * rail goes three deep. The icon says which kind it is at a glance, since at
 * that depth there is no room for a word saying so.
 */
export type SubEntry = {
	href: string;
	label: string;
	title?: string;
	icon?: Component;
	children?: SubEntry[];
};

export type PageGroup = {
	/** Named above the entries; dropped when the rail has no room for it. */
	label: MessageKey;
	pages: readonly PageDefinition[];
};

/**
 * The rail's entries, in the three groups they fall into.
 *
 * What is happening, what has happened, and what should happen next — the same
 * order as before, with the seams named.
 */
export const PAGE_GROUPS: readonly PageGroup[] = [
	{ label: 'nav.group.now', pages: PAGES.slice(0, 1) },
	{ label: 'nav.group.past', pages: PAGES.slice(1, 7) },
	{ label: 'nav.group.next', pages: PAGES.slice(7) }
];

/** Shown in the rail only while the log view is switched on in settings. */
export const LOG_PAGE: PageDefinition = {
	href: '/logs',
	label: 'nav.logs',
	description: 'nav.logs.description',
	icon: Logs
};

/** Pinned at the foot of the rail, above the settings. */
export const INFO_PAGE: PageDefinition = {
	href: '/info',
	label: 'nav.info',
	description: 'nav.info.description',
	icon: Info
};

export const SETTINGS_PAGE: PageDefinition = {
	href: '/settings',
	label: 'nav.settings',
	description: 'nav.settings.description',
	icon: Settings
};

// The log page is in here even while the rail hides it: the breadcrumb has
// to be able to name a page the user reached by its address.
export const ALL_PAGES: readonly PageDefinition[] = [...PAGES, LOG_PAGE, INFO_PAGE, SETTINGS_PAGE];

/** The rail highlights the deepest match, so /sessions/abc keeps Sessions active. */
export function activeHref(pathname: string): Pathname {
	const match = ALL_PAGES.filter(
		(page) => pathname === page.href || (page.href !== '/' && pathname.startsWith(page.href))
	).sort((a, b) => b.href.length - a.href.length)[0];
	return match?.href ?? '/';
}

export function pageFor(pathname: string): PageDefinition {
	const href = activeHref(pathname);
	return ALL_PAGES.find((page) => page.href === href) ?? ALL_PAGES[0];
}
