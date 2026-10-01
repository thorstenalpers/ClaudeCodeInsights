<script module lang="ts">
	/**
	 * One figure, with its own breakdown underneath.
	 *
	 * The sublabel is the point of this: the same strip that says "4.1M tokens"
	 * also says how those split across input, cache and output, without a hover
	 * and without a second row of cells.
	 */
	export type Vital = {
		label: string;
		value: string;
		sublabel?: string;
		/** Shown on hover; the cell is plain text when there is nothing to say. */
		info?: string;
		/** Draws attention to a figure that means something went differently. */
		warn?: boolean;
	};
</script>

<script lang="ts">
	import * as Tooltip from '$lib/components/ui/tooltip';

	type Props = { vitals: Vital[] };
	let { vitals }: Props = $props();
</script>

<div class="flex flex-wrap rounded-md border">
	{#each vitals as vital (vital.label)}
		<div class="min-w-[7rem] flex-1 border-r px-3 py-2 last:border-r-0">
			{#if vital.info}
				<Tooltip.Root>
					<Tooltip.Trigger class="text-left">
						{#snippet child({ props })}
							<div {...props} class="cursor-help">
								<div class="text-[11px] tracking-wide text-muted-foreground uppercase">
									{vital.label}
								</div>
								<div
									class={[
										'text-base font-semibold tabular-nums',
										vital.warn ? 'text-amber-600' : 'text-foreground'
									]}
								>
									{vital.value}
								</div>
								{#if vital.sublabel}
									<div class="text-[10px] leading-tight text-muted-foreground">
										{vital.sublabel}
									</div>
								{/if}
							</div>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content class="max-w-xs">{vital.info}</Tooltip.Content>
				</Tooltip.Root>
			{:else}
				<div class="text-[11px] tracking-wide text-muted-foreground uppercase">{vital.label}</div>
				<div
					class={[
						'text-base font-semibold tabular-nums',
						vital.warn ? 'text-amber-600' : 'text-foreground'
					]}
				>
					{vital.value}
				</div>
				{#if vital.sublabel}
					<div class="text-[10px] leading-tight text-muted-foreground">{vital.sublabel}</div>
				{/if}
			{/if}
		</div>
	{/each}
</div>
