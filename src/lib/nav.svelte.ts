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

  /**
   * The session whose transcript is open, if any.
   *
   * Detail is a layer over the page rather than a page of its own, so the
   * sessions table underneath keeps its scroll position, sorting and filters
   * and going back lands exactly where the user left.
   */
  detailSessionId = $state<string | null>(null);
  detailLabel = $state<string>('');

  go(key: PageKey): void {
    this.detailSessionId = null;
    this.mounted.add(key);
    this.active = key;
  }

  openSession(sessionId: string, label: string): void {
    this.detailSessionId = sessionId;
    this.detailLabel = label;
  }

  closeDetail(): void {
    this.detailSessionId = null;
  }
}

export const nav = new Nav();
