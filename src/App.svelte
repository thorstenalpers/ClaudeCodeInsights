<script lang="ts">
  import { ModeWatcher } from 'mode-watcher';
  import AppSidebar from '$lib/components/app-sidebar.svelte';
  import PageHost from '$lib/components/page-host.svelte';
  import ThemeMenu from '$lib/components/theme-menu.svelte';
  import * as Sidebar from '$lib/components/ui/sidebar';
  import { Separator } from '$lib/components/ui/separator';
  import { nav } from '$lib/nav.svelte';
  import { PAGES, SETTINGS_PAGE, pageFor } from '$lib/pages';
  import { theme } from '$lib/theme.svelte';

  theme.init();

  const current = $derived(pageFor(nav.active));
</script>

<ModeWatcher />

<Sidebar.Provider>
  <AppSidebar
    pages={PAGES}
    settingsPage={SETTINGS_PAGE}
    active={nav.active}
    onselect={(key) => nav.go(key)}
  />

  <Sidebar.Inset class="flex h-screen min-w-0 flex-col overflow-hidden">
    <header class="flex h-14 shrink-0 items-center gap-2 border-b px-3">
      <Sidebar.Trigger />
      <Separator orientation="vertical" class="mr-1 h-4" />
      <div class="flex min-w-0 flex-col">
        <h1 class="truncate text-sm leading-tight font-semibold">{current.label}</h1>
        <p class="text-muted-foreground truncate text-xs leading-tight">{current.description}</p>
      </div>
      <div class="ml-auto flex items-center gap-1">
        <ThemeMenu />
      </div>
    </header>

    <main class="relative min-h-0 flex-1">
      <PageHost />
    </main>
  </Sidebar.Inset>
</Sidebar.Provider>
