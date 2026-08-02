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
import Tools from '@lucide/svelte/icons/wrench';

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
		href: '/assistant',
		label: 'nav.assistant',
		description: 'nav.assistant.description',
		icon: Assistant
	}
];

export const SETTINGS_PAGE: PageDefinition = {
	href: '/settings',
	label: 'nav.settings',
	description: 'nav.settings.description',
	icon: Settings
};

export const ALL_PAGES: readonly PageDefinition[] = [...PAGES, SETTINGS_PAGE];

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
