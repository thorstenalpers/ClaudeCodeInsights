<script lang="ts">
  import type { Component } from 'svelte';

  type Props = {
    icon: Component;
    title: string;
    description: string;
    /** What this page will show once the data layer exists. */
    planned: string[];
  };

  let { icon: Icon, title, description, planned }: Props = $props();
</script>

<div class="grid h-full place-items-center p-8">
  <div class="flex max-w-md flex-col items-center gap-4 text-center">
    <div class="bg-muted text-muted-foreground grid size-12 place-items-center rounded-lg">
      <Icon class="size-6" />
    </div>

    <div class="flex flex-col gap-1">
      <h2 class="text-lg font-medium">{title}</h2>
      <p class="text-muted-foreground text-sm">{description}</p>
    </div>

    <ul class="text-muted-foreground flex flex-col gap-1 text-sm">
      {#each planned as item (item)}
        <li>{item}</li>
      {/each}
    </ul>

    <p class="text-muted-foreground text-xs">Waiting on the transcript scanner.</p>
  </div>
</div>
