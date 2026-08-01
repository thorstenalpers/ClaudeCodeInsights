import type { Component } from 'svelte';
import Activity from '@lucide/svelte/icons/calendar-clock';
import Agents from '@lucide/svelte/icons/bot';
import Assistant from '@lucide/svelte/icons/sparkles';
import Cost from '@lucide/svelte/icons/circle-dollar-sign';
import Overview from '@lucide/svelte/icons/layout-dashboard';
import Projects from '@lucide/svelte/icons/folder-git-2';
import Sessions from '@lucide/svelte/icons/messages-square';
import Settings from '@lucide/svelte/icons/settings';
import Tools from '@lucide/svelte/icons/wrench';

import type { PageKey } from './nav.svelte';
import ActivityView from '../views/activity-view.svelte';
import AgentsView from '../views/agents-view.svelte';
import AssistantView from '../views/assistant-view.svelte';
import CostView from '../views/cost-view.svelte';
import OverviewView from '../views/overview-view.svelte';
import ProjectsView from '../views/projects-view.svelte';
import SessionsView from '../views/sessions-view.svelte';
import SettingsView from '../views/settings-view.svelte';
import ToolsView from '../views/tools-view.svelte';

export type PageDefinition = {
  key: PageKey;
  label: string;
  /** Shown under the page title; says what the page answers. */
  description: string;
  icon: Component;
  component: Component;
};

/** The pages reachable from the rail, in order. Settings is pinned separately. */
export const PAGES: readonly PageDefinition[] = [
  {
    key: 'overview',
    label: 'Overview',
    description: 'Tokens, cost and activity at a glance',
    icon: Overview,
    component: OverviewView,
  },
  {
    key: 'sessions',
    label: 'Sessions',
    description: 'Every conversation, with its tokens and cost',
    icon: Sessions,
    component: SessionsView,
  },
  {
    key: 'cost',
    label: 'Cost & Models',
    description: 'What each model costs, and why',
    icon: Cost,
    component: CostView,
  },
  {
    key: 'projects',
    label: 'Projects',
    description: 'Usage per project and branch',
    icon: Projects,
    component: ProjectsView,
  },
  {
    key: 'activity',
    label: 'Activity',
    description: 'When the work actually happens',
    icon: Activity,
    component: ActivityView,
  },
  {
    key: 'agents',
    label: 'Agents',
    description: 'Subagent runs and what they cost',
    icon: Agents,
    component: AgentsView,
  },
  {
    key: 'tools',
    label: 'Tools',
    description: 'Which tools consume the tokens',
    icon: Tools,
    component: ToolsView,
  },
  {
    key: 'assistant',
    label: 'Assistant',
    description: 'Ask questions about your own usage',
    icon: Assistant,
    component: AssistantView,
  },
];

export const SETTINGS_PAGE: PageDefinition = {
  key: 'settings',
  label: 'Settings',
  description: 'Scan paths, appearance, API keys',
  icon: Settings,
  component: SettingsView,
};

export const ALL_PAGES: readonly PageDefinition[] = [...PAGES, SETTINGS_PAGE];

export function pageFor(key: PageKey): PageDefinition {
  const page = ALL_PAGES.find((p) => p.key === key);
  if (!page) throw new Error(`No page registered for '${key}'`);
  return page;
}
