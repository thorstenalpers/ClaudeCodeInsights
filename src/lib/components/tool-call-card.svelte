<script lang="ts">
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import type { ToolCall } from '$lib/api';
  import { Badge } from '$lib/components/ui/badge';

  type Props = { call: ToolCall };
  let { call }: Props = $props();

  let open = $state(false);

  // A one-line gist so a collapsed call still says what it did.
  const summary = $derived(call.input.replace(/\s+/g, ' ').trim().slice(0, 120));
</script>

<div class="rounded-md border text-sm">
  <button
    type="button"
    class="hover:bg-accent/50 flex w-full items-center gap-2 px-3 py-2 text-left transition-colors"
    onclick={() => (open = !open)}
  >
    <ChevronRight class="text-muted-foreground size-3.5 shrink-0 {open ? 'rotate-90' : ''}" />
    <Badge variant="outline" class="font-mono text-xs font-normal">{call.name}</Badge>
    {#if call.isError}
      <TriangleAlert class="text-destructive size-3.5 shrink-0" />
    {/if}
    <span class="text-muted-foreground truncate font-mono text-xs">{summary}</span>
  </button>

  {#if open}
    <div class="flex flex-col gap-2 border-t px-3 py-2">
      <div class="flex flex-col gap-1">
        <span class="text-muted-foreground text-xs font-medium">Input</span>
        <pre
          class="bg-muted max-h-64 overflow-auto rounded p-2 font-mono text-xs whitespace-pre-wrap">{call.input ||
            '—'}{call.inputTruncated ? '\n…' : ''}</pre>
      </div>

      {#if call.result !== null}
        <div class="flex flex-col gap-1">
          <span class="text-muted-foreground text-xs font-medium">
            Result{call.isError ? ' (error)' : ''}
          </span>
          <pre
            class="bg-muted max-h-64 overflow-auto rounded p-2 font-mono text-xs whitespace-pre-wrap">{call.result ||
              '—'}{call.resultTruncated ? '\n…' : ''}</pre>
        </div>
      {:else}
        <span class="text-muted-foreground text-xs">No result recorded for this call.</span>
      {/if}
    </div>
  {/if}
</div>
