<script lang="ts">
	/**
	 * One project, in as many views as it has sides to it.
	 *
	 * The tabs are routes rather than a switch in one page: each side loads what
	 * it needs and nothing else, and a reload lands where the reader was.
	 */
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { displayPath } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';

	let { children }: { children: Snippet } = $props();

	// SvelteKit hands the parameter over decoded; a link back into the URL has to
	// encode it again, or a Windows path splits into segments and matches nothing.
	const projectPath = $derived(page.params.path ?? '');
	const encoded = $derived(encodeURIComponent(projectPath));

	/** The last segment names the tab; the bare project path is the first one. */
	const active = $derived.by(() => {
		const tail = page.url.pathname.split('/').at(-1) ?? '';
		return ['time', 'sessions', 'files'].includes(tail) ? tail : 'overview';
	});

	function tabClass(id: string): string {
		return [
			'rounded-md px-2.5 py-1 text-sm transition-colors',
			active === id
				? 'bg-primary text-primary-foreground'
				: 'text-muted-foreground hover:bg-accent hover:text-foreground'
		].join(' ');
	}
</script>

<div class="flex h-full flex-col">
	<div class="flex shrink-0 flex-col gap-2 border-b px-4 py-3">
		<p class="truncate font-mono text-xs text-muted-foreground" title={displayPath(projectPath)}>
			{displayPath(projectPath)}
		</p>
		<nav class="flex flex-wrap gap-1" aria-label={t('nav.projects')}>
			<a href={resolve('/projects/[path]', { path: encoded })} class={tabClass('overview')}>
				{t('projects.detail.overview')}
			</a>
			<a href={resolve('/projects/[path]/time', { path: encoded })} class={tabClass('time')}>
				{t('projects.detail.time')}
			</a>
			<a
				href={resolve('/projects/[path]/sessions', { path: encoded })}
				class={tabClass('sessions')}
			>
				{t('projects.detail.sessions')}
			</a>
			<a href={resolve('/projects/[path]/files', { path: encoded })} class={tabClass('files')}>
				{t('projects.detail.files')}
			</a>
		</nav>
	</div>

	<div class="min-h-0 flex-1 overflow-auto">
		{@render children()}
	</div>
</div>
