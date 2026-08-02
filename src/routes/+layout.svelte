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
	import AssistantSheet from '$lib/components/assistant-sheet.svelte';
	import StatusBar from '$lib/components/status-bar.svelte';
	import { Button } from '$lib/components/ui/button';
	import ScanButton from '$lib/components/scan-button.svelte';
	import LanguageMenu from '$lib/components/language-menu.svelte';
	import ModeToggle from '$lib/components/mode-toggle.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Separator } from '$lib/components/ui/separator';
	import { i18n, t } from '$lib/i18n/index.svelte';
	import { isHosted } from '$lib/ipc.svelte';
	import { nav } from '$lib/nav.svelte';
	import { PAGES, SETTINGS_PAGE, activeHref, pageFor } from '$lib/pages';
	import { scan } from '$lib/scan.svelte';
	import { theme } from '$lib/theme.svelte';
	import '../app.css';

	let { children }: { children: Snippet } = $props();

	/** Below this the rail alone is worth more than the labels beside it. */
	const NARROW = 1100;

	let viewport = $state(NARROW);
	/** What the user chose while there was room; restored when there is again. */
	let preferred = $state(true);

	// The width decides, not the last click: a window dragged narrow collapses,
	// and dragged wide again comes back to whatever the user had set.
	let sidebarOpen = $derived(viewport < NARROW ? false : preferred);

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
	void scan.init();

	$effect(() => {
		// Two nested frames: the first is scheduled before the upcoming paint,
		// the second only runs after it, so the overlay lifts on real content.
		requestAnimationFrame(() => requestAnimationFrame(() => (starting = false)));
	});

	const current = $derived(pageFor(page.url.pathname));
	const detailSessionId = $derived(page.params.id ?? null);
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
		sidebarOpen = open;
		if (viewport >= NARROW) preferred = open;
	}}
>
	<AppSidebar pages={PAGES} settingsPage={SETTINGS_PAGE} active={activeHref(page.url.pathname)} />

	<Sidebar.Inset class="flex h-screen min-w-0 flex-col overflow-hidden">
		<header class="@container flex h-12 shrink-0 items-center gap-2 border-b px-3">
			<AppMark class="hidden size-5 shrink-0 text-foreground @md:block" />
			<Separator orientation="vertical" class="mr-1 hidden h-4 @md:block" />

			{#if detailSessionId}
				<Button
					variant="ghost"
					size="icon"
					aria-label={t('header.back')}
					onclick={() => goto(resolve('/sessions'))}
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
					{:else}
						<span class="truncate font-semibold">{t(current.label)}</span>
					{/if}
				</div>
				<p class="hidden truncate text-xs leading-tight text-muted-foreground @xl:block">
					{detailSessionId ?? t(current.description)}
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
		<AssistantSheet />
	</Sidebar.Inset>
</Sidebar.Provider>
