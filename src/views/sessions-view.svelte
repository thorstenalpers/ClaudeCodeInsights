<script lang="ts">
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Users from '@lucide/svelte/icons/users';
  import { api, type SessionFacets, type SessionPage } from '$lib/api';
  import ActivityBadge from '$lib/components/activity-badge.svelte';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import { Input } from '$lib/components/ui/input';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import * as Table from '$lib/components/ui/table';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import { compact, exact } from '$lib/format';
  import { isHosted } from '$lib/ipc.svelte';
  import { scan } from '$lib/scan.svelte';

  type Column = {
    id: string;
    label: string;
    /** Sortable columns name the key the host understands. */
    sort?: string;
    numeric?: boolean;
    class?: string;
  };

  const COLUMNS: Column[] = [
    { id: 'topic', label: 'Session', sort: 'topic' },
    { id: 'project', label: 'Project', sort: 'project' },
    { id: 'activity', label: 'Activity', sort: 'activity', class: 'w-40' },
    { id: 'last', label: 'Last active', sort: 'last' },
    { id: 'duration', label: 'Duration', sort: 'duration', numeric: true },
    { id: 'turns', label: 'Turns', sort: 'turns', numeric: true },
    { id: 'input', label: 'Input', sort: 'input', numeric: true },
    { id: 'output', label: 'Output', sort: 'output', numeric: true },
    { id: 'cacheRead', label: 'Cache', sort: 'cacheRead', numeric: true },
    { id: 'model', label: 'Model', sort: 'model' },
  ];

  const PAGE_SIZE = 25;

  let page = $state(0);
  let sort = $state('last');
  let descending = $state(true);
  let searchInput = $state('');
  let search = $state('');
  let activities = $state<string[]>([]);

  let result = $state<SessionPage | null>(null);
  let facets = $state<SessionFacets | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // The host does the filtering, so every keystroke would be a query. Waiting
  // for a pause keeps a long history responsive while typing.
  let debounce: ReturnType<typeof setTimeout>;
  function onSearchInput() {
    clearTimeout(debounce);
    debounce = setTimeout(() => {
      search = searchInput;
      page = 0;
    }, 250);
  }

  function toggleSort(column: Column) {
    if (!column.sort) return;
    if (sort === column.sort) {
      descending = !descending;
    } else {
      sort = column.sort;
      descending = true;
    }
    page = 0;
  }

  function toggleActivity(value: string) {
    activities = activities.includes(value)
      ? activities.filter((entry) => entry !== value)
      : [...activities, value];
    page = 0;
  }

  $effect(() => {
    const query = {
      page,
      pageSize: PAGE_SIZE,
      sort,
      descending,
      search: search || null,
      activities,
    };
    void scan.dataVersion;

    if (!isHosted) {
      loading = false;
      return;
    }

    let cancelled = false;
    error = null;
    api
      .listSessions(query)
      .then((value) => {
        if (!cancelled) result = value;
      })
      .catch((cause) => {
        if (!cancelled) error = String(cause);
      })
      .finally(() => {
        if (!cancelled) loading = false;
      });

    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    void scan.dataVersion;
    if (isHosted) void api.getSessionFacets().then((value) => (facets = value));
  });

  const totalPages = $derived(result ? Math.max(1, Math.ceil(result.total / result.pageSize)) : 1);

  function formatDuration(minutes: number): string {
    if (minutes < 1) return '<1m';
    if (minutes < 60) return `${minutes}m`;
    return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
  }

  function formatWhen(iso: string | null): string {
    if (!iso) return '—';
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) return '—';
    return date.toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function shortModel(model: string | null): string {
    return model ? model.replace(/^claude-/, '') : '—';
  }
</script>

<div class="flex h-full flex-col gap-4 p-6">
  {#if !isHosted}
    <p class="text-muted-foreground text-sm">
      Running in a browser without the host, so there is no data to show.
    </p>
  {:else}
    <div class="flex shrink-0 flex-wrap items-center gap-2">
      <Input
        placeholder="Search topic, project or branch…"
        class="max-w-xs"
        bind:value={searchInput}
        oninput={onSearchInput}
      />

      {#if facets}
        <div class="flex flex-wrap items-center gap-1">
          {#each facets.activities as value (value)}
            <Button
              variant={activities.includes(value) ? 'default' : 'outline'}
              size="sm"
              class="h-7 px-2 text-xs font-normal capitalize"
              onclick={() => toggleActivity(value)}
            >
              {value}
            </Button>
          {/each}
        </div>
      {/if}

      {#if result}
        <span class="text-muted-foreground ml-auto text-xs tabular-nums">
          {exact(result.total)} sessions
        </span>
      {/if}
    </div>

    {#if error}
      <Card.Root>
        <Card.Header>
          <Card.Title>Could not load sessions</Card.Title>
          <Card.Description class="font-mono text-xs">{error}</Card.Description>
        </Card.Header>
      </Card.Root>
    {:else if loading && !result}
      <div class="flex flex-col gap-2">
        {#each { length: 8 } as _, index (index)}
          <Skeleton class="h-10 w-full" />
        {/each}
      </div>
    {:else if result && result.rows.length === 0}
      <Card.Root>
        <Card.Header>
          <Card.Title>No sessions match</Card.Title>
          <Card.Description>
            {search || activities.length > 0
              ? 'Try clearing the search or the activity filters.'
              : 'Run a scan to read your transcripts.'}
          </Card.Description>
        </Card.Header>
      </Card.Root>
    {:else if result}
      <div class="min-h-0 flex-1 overflow-auto rounded-md border">
        <Table.Root>
          <Table.Header class="bg-background sticky top-0 z-10">
            <Table.Row>
              {#each COLUMNS as column (column.id)}
                <Table.Head class={[column.class, column.numeric && 'text-right']}>
                  {#if column.sort}
                    <button
                      type="button"
                      class="hover:text-foreground inline-flex items-center gap-1 transition-colors"
                      onclick={() => toggleSort(column)}
                    >
                      {column.label}
                      {#if sort === column.sort}
                        {#if descending}
                          <ArrowDown class="size-3" />
                        {:else}
                          <ArrowUp class="size-3" />
                        {/if}
                      {/if}
                    </button>
                  {:else}
                    {column.label}
                  {/if}
                </Table.Head>
              {/each}
            </Table.Row>
          </Table.Header>

          <Table.Body>
            {#each result.rows as row (row.sessionId)}
              <Table.Row>
                <Table.Cell class="max-w-[22rem]">
                  <div class="flex items-center gap-2">
                    <span class="truncate font-medium">
                      {row.topic ?? row.sessionId.slice(0, 8)}
                    </span>
                    {#if row.hasSubagents}
                      <Tooltip.Root>
                        <Tooltip.Trigger>
                          {#snippet child({ props })}
                            <Users {...props} class="text-muted-foreground size-3.5 shrink-0" />
                          {/snippet}
                        </Tooltip.Trigger>
                        <Tooltip.Content>Dispatched subagents</Tooltip.Content>
                      </Tooltip.Root>
                    {/if}
                  </div>
                  {#if row.tags.length > 0}
                    <div class="mt-1 flex flex-wrap gap-1">
                      {#each row.tags as tag (tag)}
                        <Badge variant="outline" class="h-4 px-1 text-[10px] font-normal">
                          {tag}
                        </Badge>
                      {/each}
                    </div>
                  {/if}
                </Table.Cell>

                <Table.Cell class="text-muted-foreground max-w-[14rem]">
                  <span class="block truncate">{row.projectName ?? '—'}</span>
                  {#if row.gitBranch}
                    <span class="block truncate font-mono text-xs opacity-70">{row.gitBranch}</span>
                  {/if}
                </Table.Cell>

                <Table.Cell>
                  <ActivityBadge activity={row.activity} profile={row.profile} />
                </Table.Cell>

                <Table.Cell class="text-muted-foreground whitespace-nowrap">
                  {formatWhen(row.lastTs)}
                </Table.Cell>
                <Table.Cell class="text-right tabular-nums whitespace-nowrap">
                  {formatDuration(row.durationMinutes)}
                </Table.Cell>
                <Table.Cell class="text-right tabular-nums">{row.turnCount}</Table.Cell>
                <Table.Cell class="text-right tabular-nums" title={exact(row.inputTokens)}>
                  {compact(row.inputTokens)}
                </Table.Cell>
                <Table.Cell class="text-right tabular-nums" title={exact(row.outputTokens)}>
                  {compact(row.outputTokens)}
                </Table.Cell>
                <Table.Cell class="text-right tabular-nums" title={exact(row.cacheReadTokens)}>
                  {compact(row.cacheReadTokens)}
                </Table.Cell>
                <Table.Cell class="text-muted-foreground whitespace-nowrap">
                  {shortModel(row.model)}
                </Table.Cell>
              </Table.Row>
            {/each}
          </Table.Body>
        </Table.Root>
      </div>

      <div class="flex shrink-0 items-center justify-between">
        <span class="text-muted-foreground text-xs tabular-nums">
          Page {result.page + 1} of {totalPages}
        </span>
        <div class="flex gap-1">
          <Button
            variant="outline"
            size="icon"
            class="size-7"
            disabled={result.page === 0}
            onclick={() => (page = Math.max(0, page - 1))}
            aria-label="Previous page"
          >
            <ChevronLeft />
          </Button>
          <Button
            variant="outline"
            size="icon"
            class="size-7"
            disabled={result.page + 1 >= totalPages}
            onclick={() => (page = page + 1)}
            aria-label="Next page"
          >
            <ChevronRight />
          </Button>
        </div>
      </div>
    {/if}
  {/if}
</div>
