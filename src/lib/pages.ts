import type { Component } from 'svelte';
import type { Pathname } from '$app/types';
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
	label: string;
	/** Shown under the page title; says what the page answers. */
	description: string;
	icon: Component;
};

/** The pages reachable from the rail, in order. Settings is pinned separately. */
export const PAGES: readonly PageDefinition[] = [
	{
		href: '/',
		label: 'Overview',
		description: 'Tokens, cost and activity at a glance',
		icon: Overview
	},
	{
		href: '/sessions',
		label: 'Sessions',
		description: 'Every conversation, with its tokens and cost',
		icon: Sessions
	},
	{
		href: '/cost',
		label: 'Cost & Models',
		description: 'What each model costs, and why',
		icon: Cost
	},
	{
		href: '/projects',
		label: 'Projects',
		description: 'Usage per project and branch',
		icon: Projects
	},
	{
		href: '/activity',
		label: 'Activity',
		description: 'When the work actually happens',
		icon: Activity
	},
	{
		href: '/agents',
		label: 'Agents',
		description: 'Subagent runs and what they cost',
		icon: Agents
	},
	{
		href: '/tools',
		label: 'Tools',
		description: 'Which tools consume the tokens',
		icon: Tools
	},
	{
		href: '/assistant',
		label: 'Assistant',
		description: 'Ask questions about your own usage',
		icon: Assistant
	}
];

export const SETTINGS_PAGE: PageDefinition = {
	href: '/settings',
	label: 'Settings',
	description: 'Scan paths, appearance, API keys',
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
