<script lang="ts">
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import Info from '@lucide/svelte/icons/info';
	import Filter from '@lucide/svelte/icons/list-filter';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import * as Table from '$lib/components/ui/table';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { t } from '$lib/i18n/index.svelte';
	import type { FilterKind } from '$lib/table.svelte';

	type Props = {
		id: string;
		label: string;
		/** 'asc', 'desc', or null when this column does not sort. */
		direction: 'asc' | 'desc' | null;
		/** Position in the sort order; only shown once a second column joins. */
		rank: number;
		multi: boolean;
		kind: FilterKind;
		filtered: boolean;
		options: string[];
		chosen: string[];
		text: string;
		range: { min: string; max: string };
		numeric?: boolean;
		class?: string;
		/** A sentence or three on what the column actually measures. */
		info?: string;
		onsort: (id: string, additive: boolean) => void;
		ontoggle: (id: string, value: string) => void;
		ontext: (id: string, value: string) => void;
		onrange: (id: string, bound: 'min' | 'max', value: string) => void;
		onclear: (id: string) => void;
	};

	let {
		id,
		label,
		direction,
		rank,
		multi,
		kind,
		filtered,
		options,
		chosen,
		text,
		range,
		numeric = false,
		class: className,
		info,
		onsort,
		ontoggle,
		ontext,
		onrange,
		onclear
	}: Props = $props();

	function valueOf(event: Event): string {
		return (event.currentTarget as HTMLInputElement).value;
	}
</script>

<Table.Head class={[className, 'h-8']}>
	<div class={['flex items-center gap-1', numeric && 'justify-end']}>
		<button
			type="button"
			class="inline-flex items-center gap-1 transition-colors hover:text-foreground"
			title={t('common.multiSortHint')}
			onclick={(event) => onsort(id, event.shiftKey)}
		>
			{label}
			{#if direction === 'desc'}
				<ArrowDown class="size-3" />
			{:else if direction === 'asc'}
				<ArrowUp class="size-3" />
			{/if}
			{#if direction && multi}
				<span class="text-[10px] text-muted-foreground tabular-nums">{rank}</span>
			{/if}
		</button>

		{#if info}
			<Tooltip.Provider delayDuration={200}>
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<button
								{...props}
								type="button"
								aria-label={`${label} — ${t('common.whatIsThis')}`}
								class="rounded p-0.5 text-muted-foreground/50 transition-colors hover:text-foreground"
							>
								<Info class="size-3" />
							</button>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content class="max-w-72 text-xs font-normal">{info}</Tooltip.Content>
				</Tooltip.Root>
			</Tooltip.Provider>
		{/if}

		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<button
						{...props}
						type="button"
						aria-label={`${label} — ${t('common.filter')}`}
						class={[
							'rounded p-0.5 transition-colors hover:text-foreground',
							filtered ? 'text-primary' : 'text-muted-foreground/60'
						]}
					>
						<Filter class="size-3" />
					</button>
				{/snippet}
			</DropdownMenu.Trigger>

			<DropdownMenu.Content class="max-h-80 w-56 overflow-y-auto">
				<DropdownMenu.Label class="flex items-center justify-between gap-2">
					{t('common.filter')}
					{#if filtered}
						<button
							type="button"
							class="text-xs font-normal text-muted-foreground hover:text-foreground"
							onclick={() => onclear(id)}
						>
							{t('common.clearFilter')}
						</button>
					{/if}
				</DropdownMenu.Label>
				<DropdownMenu.Separator />

				{#if kind === 'range'}
					<div class="flex items-center gap-2 p-2">
						<input
							type="number"
							value={range.min}
							placeholder={t('common.from')}
							aria-label={`${label} — ${t('common.from')}`}
							class="h-7 w-full rounded border bg-transparent px-1.5 text-xs"
							oninput={(event) => onrange(id, 'min', valueOf(event))}
						/>
						<span class="text-xs text-muted-foreground">–</span>
						<input
							type="number"
							value={range.max}
							placeholder={t('common.to')}
							aria-label={`${label} — ${t('common.to')}`}
							class="h-7 w-full rounded border bg-transparent px-1.5 text-xs"
							oninput={(event) => onrange(id, 'max', valueOf(event))}
						/>
					</div>
				{:else if kind === 'list'}
					{#each options as option (option)}
						<DropdownMenu.CheckboxItem
							checked={chosen.includes(option)}
							onCheckedChange={() => ontoggle(id, option)}
							closeOnSelect={false}
						>
							<span class="truncate">{option || t('common.none')}</span>
						</DropdownMenu.CheckboxItem>
					{/each}
				{:else}
					<div class="p-2">
						<input
							type="search"
							value={text}
							placeholder={t('common.filter')}
							aria-label={`${label} — ${t('common.filter')}`}
							class="h-7 w-full rounded border bg-transparent px-1.5 text-xs"
							oninput={(event) => ontext(id, valueOf(event))}
						/>
					</div>
				{/if}
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</div>
</Table.Head>
