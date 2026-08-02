<script lang="ts">
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { ModeWatcher } from 'mode-watcher';
	import { tick, type Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import AppMark from '$lib/components/app-mark.svelte';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import { Button } from '$lib/components/ui/button';
	import ScanButton from '$lib/components/scan-button.svelte';
	import LanguageMenu from '$lib/components/language-menu.svelte';
	import ModeToggle from '$lib/components/mode-toggle.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Separator } from '$lib/components/ui/separator';
	import { i18n, t } from '$lib/i18n/index.svelte';
	import { isHosted, reportReady } from '$lib/ipc.svelte';
	import { nav } from '$lib/nav.svelte';
	import { PAGES, SETTINGS_PAGE, activeHref, pageFor } from '$lib/pages';
	import { scan } from '$lib/scan.svelte';
	import { theme } from '$lib/theme.svelte';
	import '../app.css';

	let { children }: { children: Snippet } = $props();

	theme.init();
	i18n.init();
	void scan.init();
	void signalReady();

	const current = $derived(pageFor(page.url.pathname));
	const detailSessionId = $derived(page.params.id ?? null);

	/** Resolves once a real frame has been painted, or after `fallbackMs` regardless. */
	function afterFirstPaint(fallbackMs: number): Promise<void> {
		return new Promise((resolve) => {
			// Two nested frames: the first is scheduled before the upcoming paint,
			// the second only runs after it.
			requestAnimationFrame(() => requestAnimationFrame(() => resolve()));

			// The main window starts hidden, and a window that is not compositing
			// gets its animation frames throttled to a standstill — without this
			// fallback the splash would wait forever for a frame that is not coming.
			setTimeout(resolve, fallbackMs);
		});
	}

	async function signalReady(): Promise<void> {
		if (!isHosted) return;

		await tick();
		await afterFirstPaint(500);
		await reportReady('frontend').catch(() => {
			// The host reveals the window on a timeout anyway; a failure here must
			// not strand the user on the splash screen.
		});
	}
</script>

<ModeWatcher />

<Sidebar.Provider>
	<AppSidebar pages={PAGES} settingsPage={SETTINGS_PAGE} active={activeHref(page.url.pathname)} />

	<Sidebar.Inset class="flex h-screen min-w-0 flex-col overflow-hidden">
		<header class="flex h-14 shrink-0 items-center gap-2 border-b px-3">
			<AppMark class="size-5 shrink-0 text-foreground" />
			<Separator orientation="vertical" class="mr-1 h-4" />

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
						class="shrink-0 text-muted-foreground transition-colors hover:text-foreground"
					>
						{t('app.name')}
					</a>
					<ChevronRight class="size-3 shrink-0 text-muted-foreground/60" />

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
				<p class="truncate text-xs leading-tight text-muted-foreground">
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
	</Sidebar.Inset>
</Sidebar.Provider>
