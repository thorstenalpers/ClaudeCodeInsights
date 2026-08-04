<script lang="ts">
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Folder from '@lucide/svelte/icons/folder';
	import MessagesSquare from '@lucide/svelte/icons/messages-square';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { ModeWatcher } from 'mode-watcher';
	import { fade } from 'svelte/transition';
	import type { Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import {
		api,
		type ActivityRow,
		type ModelRow,
		type ProjectRow,
		type SessionRow,
		type ToolRow
	} from '$lib/api';
	import AppMark from '$lib/components/app-mark.svelte';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import RailToggle from '$lib/components/rail-toggle.svelte';
	import StatusBar from '$lib/components/status-bar.svelte';
	import VoiceHint from '$lib/components/voice-hint.svelte';
	import { Button } from '$lib/components/ui/button';
	import ScanButton from '$lib/components/scan-button.svelte';
	import LanguageMenu from '$lib/components/language-menu.svelte';
	import ModeToggle from '$lib/components/mode-toggle.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Toaster } from '$lib/components/ui/sonner';
	import { Separator } from '$lib/components/ui/separator';
	import { displayPath } from '$lib/format';
	import type { MessageKey } from '$lib/i18n/en';
	import { i18n, t } from '$lib/i18n/index.svelte';
	import { isHosted } from '$lib/ipc.svelte';
	import { cli } from '$lib/cli.svelte';
	import { logs } from '$lib/logs.svelte';
	import { nav } from '$lib/nav.svelte';
	import {
		INFO_PAGE,
		LOG_PAGE,
		PAGE_GROUPS,
		SETTINGS_PAGE,
		activeHref,
		pageFor,
		type SubEntry
	} from '$lib/pages';
	import { scan } from '$lib/scan.svelte';
	import { theme } from '$lib/theme.svelte';
	import { voice } from '$lib/voice.svelte';
	import '../app.css';

	let { children }: { children: Snippet } = $props();

	/** Below this the rail alone is worth more than the labels beside it. */
	const NARROW = 1100;

	/**
	 * Below this the rail runs out of room downwards.
	 *
	 * Thirteen entries at the full row height need about 540 pixels once the
	 * header and the pinned foot are counted; the shorter rows fit inside the
	 * 480 the window may never go under. Measured in CSS pixels, so a phone and
	 * a scaled desktop are the same case.
	 */
	const SHORT = 620;

	let viewport = $state(NARROW);
	let viewportHeight = $state(SHORT);
	/** What the user chose while there was room; restored when there is again. */
	let preferred = $state(true);
	/** The last click, which outranks the width until the width class changes. */
	let clicked = $state<boolean | null>(null);

	const narrow = $derived(viewport < NARROW);
	const short = $derived(viewportHeight < SHORT);

	// Crossing the threshold is a new situation, so the old click stops speaking
	// for it: dragged narrow the rail wins, dragged wide again what was set does.
	$effect(() => {
		void narrow;
		clicked = null;
	});

	let sidebarOpen = $derived(clicked ?? (narrow ? false : preferred));

	const RAIL_WIDTH_KEY = 'claudeadmin.railWidth';
	/** Narrower and the labels are gone anyway; wider and the rail takes more
	 *  than it gives back. */
	const RAIL = { min: 180, max: 420, default: 256 };

	let railWidth = $state(readRailWidth());

	function readRailWidth(): number {
		if (typeof localStorage === 'undefined') return RAIL.default;
		const stored = Number(localStorage.getItem(RAIL_WIDTH_KEY));
		return Number.isFinite(stored) && stored >= RAIL.min && stored <= RAIL.max
			? stored
			: RAIL.default;
	}

	function setRailWidth(next: number, settled: boolean): void {
		railWidth = Math.min(RAIL.max, Math.max(RAIL.min, Math.round(next)));
		if (settled) localStorage.setItem(RAIL_WIDTH_KEY, String(railWidth));
	}

	/**
	 * The splash, in the window it is covering.
	 *
	 * It used to be a second Tauri window with its own WebView, which meant a
	 * whole browser engine started, painted one logo and shut down again. An
	 * overlay in the window that is already starting costs a div.
	 */
	let starting = $state(isHosted);

	theme.init();
	i18n.init();
	voice.init();
	// The folder is re-read at startup, so a pack copied in by hand counts.
	void voice.loadPacks();
	void scan.init();
	// The host's own lines, kept whether or not the view is on: a log that only
	// starts recording once someone opens it has already missed the interesting part.
	void logs.listen();
	void cli.refresh();

	$effect(() => {
		// Two nested frames: the first is scheduled before the upcoming paint,
		// the second only runs after it, so the overlay lifts on real content.
		requestAnimationFrame(() => requestAnimationFrame(() => (starting = false)));
	});

	// The log sits with the other two that are about the app rather than about
	// the work, directly above the info it is usually opened next to.
	const footerPages = $derived(
		logs.enabled ? [INFO_PAGE, LOG_PAGE, SETTINGS_PAGE] : [INFO_PAGE, SETTINGS_PAGE]
	);
	/**
	 * What hangs under each rail entry: the things this machine has, not views
	 * of them. Projects come from the same `~/.claude.json` the projects page
	 * lists; the rest from the scan.
	 *
	 * Capped and sorted by weight, because the rail is a way in rather than an
	 * inventory: past a dozen the list is longer than the navigation above it.
	 */
	const SUB_LIMIT = 12;
	/** Per project, under it. Deeper than this the rail is a transcript index. */
	const SESSIONS_PER_PROJECT = 5;

	let knownProjects = $state<ProjectRow[]>([]);
	let knownActivities = $state<ActivityRow[]>([]);
	let knownModels = $state<ModelRow[]>([]);
	let knownTools = $state<ToolRow[]>([]);
	let knownSessions = $state<SessionRow[]>([]);

	$effect(() => {
		void scan.dataVersion;
		if (!isHosted) return;
		void api
			.listProjects()
			.then((report) => (knownProjects = report.projects))
			.catch(() => (knownProjects = []));
		void api
			.listActivities()
			.then((rows) => (knownActivities = rows))
			.catch(() => (knownActivities = []));
		void api
			.listModels()
			.then((rows) => (knownModels = rows))
			.catch(() => (knownModels = []));
		void api
			.listTools()
			.then((rows) => (knownTools = rows))
			.catch(() => (knownTools = []));
		void api
			// Enough to reach every project's newest few; the rail shows a
			// handful per project, not the page's worth.
			.listSessions({ page: 1, pageSize: 200, sort: 'lastTs', descending: true })
			.then((found) => (knownSessions = found.rows))
			.catch(() => (knownSessions = []));
	});

	/** The last folder of a path: the structure above it is not a name. */
	function folderName(path: string): string {
		const parts = displayPath(path).split(/[\\/]/).filter(Boolean);
		return parts.at(-1) ?? path;
	}

	/** Same directory, two spellings, one entry: `\` and `/` reach the same place. */
	const sameProject = (path: string) => path.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase();

	/** The newest sessions of one project, by the path they carry. */
	function sessionsOf(project: string): SubEntry[] {
		const key = sameProject(project);
		return knownSessions
			.filter((row) => row.projectName && sameProject(row.projectName) === key)
			.slice(0, SESSIONS_PER_PROJECT)
			.map((row) => ({
				href: `/sessions/${encodeURIComponent(row.sessionId)}`,
				label: row.topic || row.sessionId.slice(0, 8),
				title: row.topic ?? row.sessionId,
				icon: MessagesSquare
			}));
	}

	const uniqueProjects = $derived.by(() => {
		// A plain object, rebuilt on every run and never observed, which is what
		// the reactive Map is for.
		const seen: Record<string, ProjectRow> = {};
		for (const project of knownProjects) {
			if (!project.registered && project.sessions === 0) continue;
			const key = sameProject(project.path);
			// The registered spelling wins; among equals the busier one.
			const known = seen[key];
			if (
				!known ||
				(project.registered && !known.registered) ||
				project.sessions > known.sessions
			) {
				seen[key] = project;
			}
		}
		return Object.values(seen).slice(0, SUB_LIMIT);
	});

	const subs = $derived({
		'/projects': uniqueProjects.map((project) => ({
			href: `/projects/${encodeURIComponent(project.path)}`,
			label: folderName(project.path),
			title: displayPath(project.path),
			icon: Folder,
			children: sessionsOf(project.path)
		})),
		'/activity': knownActivities.slice(0, SUB_LIMIT).map((row) => ({
			href: `/activity/${encodeURIComponent(row.activity)}`,
			label: activityLabel(row.activity)
		})),
		// The models have no page of their own: the entry narrows the one page
		// they all live on, and the first entry is that page whole.
		'/cost':
			knownModels.length === 0
				? []
				: [
						{ href: '/cost', label: t('nav.cost.all') },
						...knownModels.slice(0, SUB_LIMIT).map((row) => ({
							href: `/cost?model=${encodeURIComponent(row.model)}`,
							label: row.model.replace(/^claude-/, ''),
							title: row.model
						}))
					],
		'/tools': knownTools.slice(0, SUB_LIMIT).map((row) => ({
			href: `/tools?tool=${encodeURIComponent(row.name)}`,
			label: row.name
		}))
	});

	const current = $derived(pageFor(page.url.pathname));
	const detailSessionId = $derived(page.params.id ?? null);
	/** The activity a detail page is about, taken from the URL like the rest. */
	const detailActivity = $derived(page.params.name ?? null);
	/** The project a sub-page is about, which the URL carries but no page said. */
	const detailProject = $derived(page.params.path ?? null);
	/** The folder alone: the whole path is already on the line below. */
	const projectName = $derived(
		detailProject?.split(/[\\/]/).filter(Boolean).at(-1) ?? detailProject
	);

	/** The activity's own name where there is one, the raw id where there is not. */
	function activityLabel(activity: string): string {
		const key = `activity.${activity}` as MessageKey;
		const named = t(key);
		return named === key ? activity : named;
	}
</script>

<svelte:window bind:innerWidth={viewport} bind:innerHeight={viewportHeight} />

{#if starting}
	<div
		class="fixed inset-0 z-50 flex flex-col items-center justify-center gap-3 bg-background"
		out:fade={{ duration: 150 }}
	>
		<AppMark class="size-12 text-foreground" />
		<p class="text-sm text-muted-foreground">{t('app.name')}</p>
	</div>
{/if}

<ModeWatcher />

<!-- Bound rather than passed: the provider writes to `open` when the trigger is
     clicked, and a one-way prop stops following the width after it does. -->
<Sidebar.Provider
	style="--sidebar-width: {railWidth}px"
	bind:open={sidebarOpen}
	onOpenChange={(open: boolean) => {
		clicked = open;
		if (!narrow) preferred = open;
	}}
>
	<AppSidebar
		groups={PAGE_GROUPS}
		footer={footerPages}
		active={activeHref(page.url.pathname)}
		path={page.url.pathname}
		width={railWidth}
		onResize={setRailWidth}
		compact={short}
		{subs}
	/>

	<Sidebar.Inset class="flex h-dvh min-w-0 flex-col overflow-hidden">
		<header class="@container flex h-12 shrink-0 items-center gap-2 border-b px-3">
			<!-- Only where the rail is a sheet: there its own toggle travels inside it,
			     so this is the one way back to the navigation. -->
			<RailToggle class="md:hidden" />
			<AppMark class="hidden size-5 shrink-0 text-foreground @md:block" />
			<Separator orientation="vertical" class="mr-1 hidden h-4 @md:block" />

			{#if detailSessionId || detailProject || detailActivity}
				<Button
					variant="ghost"
					size="icon"
					aria-label={t('header.back')}
					onclick={() =>
						goto(
							detailSessionId
								? resolve('/sessions')
								: detailActivity
									? resolve('/activity')
									: resolve('/projects')
						)}
				>
					<ArrowLeft />
				</Button>
			{/if}

			<!-- The trail says where you are and walks back up. Every step but the
			     last is a link, which is what makes it navigation rather than a label. -->
			<nav class="flex min-w-0 flex-col" aria-label={t('nav.breadcrumb')}>
				<div class="flex min-w-0 items-center gap-1.5 text-sm leading-tight">
					<a
						href={resolve('/')}
						class="hidden shrink-0 text-muted-foreground transition-colors hover:text-foreground @lg:inline"
					>
						{t('app.name')}
					</a>
					<ChevronRight class="hidden size-3 shrink-0 text-muted-foreground/60 @lg:block" />

					{#if detailSessionId}
						<a
							href={resolve('/sessions')}
							class="shrink-0 text-muted-foreground transition-colors hover:text-foreground"
						>
							{t('nav.sessions')}
						</a>
						<ChevronRight class="size-3 shrink-0 text-muted-foreground/60" />
						<span class="truncate font-semibold">{nav.detailLabel || t('header.session')}</span>
					{:else if detailActivity}
						<a
							href={resolve('/activity')}
							class="shrink-0 text-muted-foreground transition-colors hover:text-foreground"
						>
							{t('nav.activity')}
						</a>
						<ChevronRight class="size-3 shrink-0 text-muted-foreground/60" />
						<span class="truncate font-semibold">{activityLabel(detailActivity)}</span>
					{:else if detailProject}
						<a
							href={resolve('/projects')}
							class="shrink-0 text-muted-foreground transition-colors hover:text-foreground"
						>
							{t('nav.projects')}
						</a>
						<ChevronRight class="size-3 shrink-0 text-muted-foreground/60" />
						<span class="truncate font-semibold">{projectName}</span>
					{:else}
						<span class="truncate font-semibold">{t(current.label)}</span>
					{/if}
				</div>
				<p class="hidden truncate text-xs leading-tight text-muted-foreground @xl:block">
					{detailSessionId ?? (detailProject ? displayPath(detailProject) : t(current.description))}
				</p>
			</nav>
			<div class="ml-auto flex items-center gap-1">
				<ScanButton />
				<LanguageMenu />
				<ModeToggle />
			</div>
		</header>

		<main class="relative min-h-0 flex-1 overflow-auto">
			{@render children()}
		</main>

		<StatusBar />
		<VoiceHint />
		<Toaster position="top-right" />
	</Sidebar.Inset>
</Sidebar.Provider>
