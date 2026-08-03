<script lang="ts">
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { ModeWatcher } from 'mode-watcher';
	import { fade } from 'svelte/transition';
	import type { Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
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
	import { i18n, t } from '$lib/i18n/index.svelte';
	import { isHosted } from '$lib/ipc.svelte';
	import { cli } from '$lib/cli.svelte';
	import { logs } from '$lib/logs.svelte';
	import { nav } from '$lib/nav.svelte';
	import { LOG_PAGE, PAGES, SETTINGS_PAGE, activeHref, pageFor } from '$lib/pages';
	import { scan } from '$lib/scan.svelte';
	import { theme } from '$lib/theme.svelte';
	import { voice } from '$lib/voice.svelte';
	import '../app.css';

	let { children }: { children: Snippet } = $props();

	/** Below this the rail alone is worth more than the labels beside it. */
	const NARROW = 1100;

	let viewport = $state(NARROW);
	/** What the user chose while there was room; restored when there is again. */
	let preferred = $state(true);
	/** The last click, which outranks the width until the width class changes. */
	let clicked = $state<boolean | null>(null);

	const narrow = $derived(viewport < NARROW);

	// Crossing the threshold is a new situation, so the old click stops speaking
	// for it: dragged narrow the rail wins, dragged wide again what was set does.
	$effect(() => {
		void narrow;
		clicked = null;
	});

	let sidebarOpen = $derived(clicked ?? (narrow ? false : preferred));

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

	const railPages = $derived(logs.enabled ? [...PAGES, LOG_PAGE] : PAGES);
	const current = $derived(pageFor(page.url.pathname));
	const detailSessionId = $derived(page.params.id ?? null);
	/** The project a sub-page is about, which the URL carries but no page said. */
	const detailProject = $derived(page.params.path ?? null);
	/** The folder alone: the whole path is already on the line below. */
	const projectName = $derived(
		detailProject?.split(/[\\/]/).filter(Boolean).at(-1) ?? detailProject
	);
</script>

<svelte:window bind:innerWidth={viewport} />

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
	bind:open={sidebarOpen}
	onOpenChange={(open: boolean) => {
		clicked = open;
		if (!narrow) preferred = open;
	}}
>
	<AppSidebar
		pages={railPages}
		settingsPage={SETTINGS_PAGE}
		active={activeHref(page.url.pathname)}
	/>

	<Sidebar.Inset class="flex h-dvh min-w-0 flex-col overflow-hidden">
		<header class="@container flex h-12 shrink-0 items-center gap-2 border-b px-3">
			<!-- Only where the rail is a sheet: there its own toggle travels inside it,
			     so this is the one way back to the navigation. -->
			<RailToggle class="md:hidden" />
			<AppMark class="hidden size-5 shrink-0 text-foreground @md:block" />
			<Separator orientation="vertical" class="mr-1 hidden h-4 @md:block" />

			{#if detailSessionId || detailProject}
				<Button
					variant="ghost"
					size="icon"
					aria-label={t('header.back')}
					onclick={() => goto(detailSessionId ? resolve('/sessions') : resolve('/projects'))}
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
		<!-- Top centre, not top right: the scan, language and mode buttons live in
		     that corner, and an error notice stays up long enough to hide them. -->
		<Toaster position="top-center" />
	</Sidebar.Inset>
</Sidebar.Provider>
