<script lang="ts">
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import { ModeWatcher } from 'mode-watcher';
	import { tick, type Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import { Button } from '$lib/components/ui/button';
	import ScanButton from '$lib/components/scan-button.svelte';
	import ThemeMenu from '$lib/components/theme-menu.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Separator } from '$lib/components/ui/separator';
	import { isHosted, reportReady } from '$lib/ipc.svelte';
	import { nav } from '$lib/nav.svelte';
	import { PAGES, SETTINGS_PAGE, activeHref, pageFor } from '$lib/pages';
	import { scan } from '$lib/scan.svelte';
	import { theme } from '$lib/theme.svelte';
	import '../app.css';

	let { children }: { children: Snippet } = $props();

	theme.init();
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
			<Sidebar.Trigger />
			<Separator orientation="vertical" class="mr-1 h-4" />

			{#if detailSessionId}
				<Button
					variant="ghost"
					size="icon"
					aria-label="Back"
					onclick={() => goto(resolve('/sessions'))}
				>
					<ArrowLeft />
				</Button>
				<div class="flex min-w-0 flex-col">
					<h1 class="truncate text-sm leading-tight font-semibold">
						{nav.detailLabel || 'Session'}
					</h1>
					<p class="truncate font-mono text-xs leading-tight text-muted-foreground">
						{detailSessionId}
					</p>
				</div>
			{:else}
				<div class="flex min-w-0 flex-col">
					<h1 class="truncate text-sm leading-tight font-semibold">{current.label}</h1>
					<p class="truncate text-xs leading-tight text-muted-foreground">{current.description}</p>
				</div>
			{/if}
			<div class="ml-auto flex items-center gap-1">
				<ScanButton />
				<ThemeMenu />
			</div>
		</header>

		<main class="relative min-h-0 flex-1 overflow-auto">
			{@render children()}
		</main>
	</Sidebar.Inset>
</Sidebar.Provider>
