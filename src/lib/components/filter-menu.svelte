<script lang="ts">
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { t } from '$lib/i18n/index.svelte';

	type Props = {
		label: string;
		options: string[];
		chosen: string[];
		/** Turns a stored value into what the list shows. */
		display?: (value: string) => string;
		onToggle: (value: string) => void;
		onClear: () => void;
	};

	let { label, options, chosen, display = (value) => value, onToggle, onClear }: Props = $props();
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button
				{...props}
				variant={chosen.length > 0 ? 'default' : 'outline'}
				size="sm"
				class="h-8 gap-1 font-normal"
				disabled={options.length === 0}
			>
				{label}
				{#if chosen.length > 0}
					<span class="tabular-nums">{chosen.length}</span>
				{/if}
				<ChevronDown class="size-3.5 opacity-60" />
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>

	<DropdownMenu.Content class="max-h-80 w-56 overflow-y-auto">
		<DropdownMenu.Label class="flex items-center justify-between gap-2">
			{label}
			{#if chosen.length > 0}
				<button
					type="button"
					class="text-xs font-normal text-muted-foreground hover:text-foreground"
					onclick={onClear}
				>
					{t('common.clearFilter')}
				</button>
			{/if}
		</DropdownMenu.Label>
		<DropdownMenu.Separator />

		{#each options as option (option)}
			<DropdownMenu.CheckboxItem
				checked={chosen.includes(option)}
				onCheckedChange={() => onToggle(option)}
				closeOnSelect={false}
			>
				<span class="truncate">{display(option)}</span>
			</DropdownMenu.CheckboxItem>
		{/each}
	</DropdownMenu.Content>
</DropdownMenu.Root>
