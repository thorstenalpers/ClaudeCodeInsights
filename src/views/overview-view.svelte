<script lang="ts">
  import { api, type Overview } from '$lib/api';
  import * as Card from '$lib/components/ui/card';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import { compact, exact, formatDate } from '$lib/format';
  import { isHosted } from '$lib/ipc.svelte';
  import { scan } from '$lib/scan.svelte';

  let overview = $state<Overview | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

  async function load() {
    error = null;
    try {
      overview = await api.getOverview();
    } catch (cause) {
      error = String(cause);
    } finally {
      loading = false;
    }
  }

  // Refetch when a scan lands new data, rather than polling.
  $effect(() => {
    void scan.dataVersion;
    if (isHosted) void load();
    else loading = false;
  });

  const cards = $derived(
    overview
      ? [
          {
            label: 'Sessions',
            value: overview.sessions,
            hint: `${overview.activeDays} active days`,
          },
          { label: 'Turns', value: overview.turns, hint: 'assistant responses' },
          { label: 'Input', value: overview.inputTokens, hint: 'tokens sent' },
          { label: 'Output', value: overview.outputTokens, hint: 'tokens generated' },
          { label: 'Cache read', value: overview.cacheReadTokens, hint: 'served from cache' },
          { label: 'Cache write', value: overview.cacheWriteTokens, hint: 'written to cache' },
        ]
      : [],
  );
</script>

<div class="flex flex-col gap-6 p-6">
  {#if !isHosted}
    <p class="text-muted-foreground text-sm">
      Running in a browser without the host, so there is no data to show.
    </p>
  {:else if loading}
    <div class="grid grid-cols-2 gap-4 lg:grid-cols-3">
      {#each { length: 6 } as _, index (index)}
        <Skeleton class="h-28 w-full" />
      {/each}
    </div>
  {:else if error}
    <Card.Root>
      <Card.Header>
        <Card.Title>Could not read the database</Card.Title>
        <Card.Description class="font-mono text-xs">{error}</Card.Description>
      </Card.Header>
    </Card.Root>
  {:else if overview && overview.turns === 0}
    <Card.Root>
      <Card.Header>
        <Card.Title>Nothing scanned yet</Card.Title>
        <Card.Description>
          Run a scan to read your transcripts. Nothing leaves this machine.
        </Card.Description>
      </Card.Header>
    </Card.Root>
  {:else if overview}
    <div class="grid grid-cols-2 gap-4 lg:grid-cols-3">
      {#each cards as card (card.label)}
        <Card.Root>
          <Card.Header class="gap-1">
            <Card.Description>{card.label}</Card.Description>
            <Card.Title class="text-2xl tabular-nums" title={exact(card.value)}>
              {compact(card.value)}
            </Card.Title>
            <p class="text-muted-foreground text-xs">{card.hint}</p>
          </Card.Header>
        </Card.Root>
      {/each}
    </div>

    <p class="text-muted-foreground text-xs tabular-nums">
      {formatDate(overview.firstTs)} — {formatDate(overview.lastTs)}
    </p>
  {/if}
</div>
