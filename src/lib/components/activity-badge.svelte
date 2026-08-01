<script lang="ts">
  import { Badge } from '$lib/components/ui/badge';
  import * as Tooltip from '$lib/components/ui/tooltip';

  type Props = {
    activity: string;
    /** Share per tool category. Shares can sum above 1 — a tool may belong to
     *  several categories — so the bar is normalised for display. */
    profile: Record<string, number>;
  };

  let { activity, profile }: Props = $props();

  const LABELS: Record<string, string> = {
    coding: 'Coding',
    debugging: 'Debugging',
    exploration: 'Exploration',
    research: 'Research',
    planning: 'Planning',
    delegation: 'Delegation',
    ops: 'Ops',
    conversation: 'Conversation',
  };

  // Fixed order so the same category keeps the same colour across every row.
  const CATEGORY_ORDER = [
    'code_change',
    'execution',
    'exploration',
    'research',
    'planning',
    'delegation',
    'mcp',
    'other',
  ];

  const CHART_VAR = ['--chart-1', '--chart-2', '--chart-3', '--chart-4', '--chart-5'];

  const segments = $derived.by(() => {
    const entries = CATEGORY_ORDER.map((name, index) => ({
      name,
      value: profile[name] ?? 0,
      colour: `var(${CHART_VAR[index % CHART_VAR.length]})`,
    })).filter((entry) => entry.value > 0);

    const total = entries.reduce((sum, entry) => sum + entry.value, 0);
    return total > 0
      ? entries.map((entry) => ({ ...entry, percent: (entry.value / total) * 100 }))
      : [];
  });
</script>

<Tooltip.Root>
  <Tooltip.Trigger>
    {#snippet child({ props })}
      <div {...props} class="flex w-full flex-col gap-1">
        <Badge variant="secondary" class="w-fit font-normal">
          {LABELS[activity] ?? activity}
        </Badge>
        {#if segments.length > 0}
          <div class="bg-muted flex h-1 w-full overflow-hidden rounded-full">
            {#each segments as segment (segment.name)}
              <div style="width: {segment.percent}%; background: {segment.colour}"></div>
            {/each}
          </div>
        {/if}
      </div>
    {/snippet}
  </Tooltip.Trigger>
  <Tooltip.Content side="left" class="max-w-56">
    {#if segments.length > 0}
      <div class="flex flex-col gap-0.5 text-xs">
        {#each segments as segment (segment.name)}
          <div class="flex items-center justify-between gap-3">
            <span class="flex items-center gap-1.5">
              <span
                class="size-2 shrink-0 rounded-[2px]"
                style="background: {segment.colour}"
              ></span>
              {segment.name.replace('_', ' ')}
            </span>
            <span class="tabular-nums">{Math.round(segment.percent)}%</span>
          </div>
        {/each}
      </div>
    {:else}
      <span class="text-xs">No tool calls</span>
    {/if}
  </Tooltip.Content>
</Tooltip.Root>
