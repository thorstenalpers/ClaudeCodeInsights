import { SvelteSet } from 'svelte/reactivity';

export type PageKey =
  | 'overview'
  | 'sessions'
  | 'cost'
  | 'projects'
  | 'activity'
  | 'agents'
  | 'tools'
  | 'assistant'
  | 'settings';

/**
 * Which page is showing, and which pages have ever been visited.
 *
 * The two are separate on purpose. A page is created on its first visit and then
 * never destroyed; switching only toggles visibility. That is what makes scroll
 * position, table sorting and chat scrollback survive navigation without every
 * view having to rebuild that itself — see the page host.
 */
class Nav {
  active = $state<PageKey>('overview');
  readonly mounted = new SvelteSet<PageKey>(['overview']);

  go(key: PageKey): void {
    this.mounted.add(key);
    this.active = key;
  }
}

export const nav = new Nav();
