<script lang="ts">
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { t } from '$lib/i18n/index.svelte';
	import { FAMILIES, rates, type FamilyId, type Rate } from '$lib/pricing.svelte';

	const FIELDS: {
		id: keyof Rate;
		label:
			| 'cost.column.input'
			| 'cost.column.output'
			| 'cost.column.cacheRead'
			| 'cost.column.cacheWrite';
	}[] = [
		{ id: 'input', label: 'cost.column.input' },
		{ id: 'output', label: 'cost.column.output' },
		{ id: 'cacheRead', label: 'cost.column.cacheRead' },
		{ id: 'cacheWrite', label: 'cost.column.cacheWrite' }
	];

	const edited = $derived(FAMILIES.some((family) => rates.isEdited(family.id)));

	function onRate(family: FamilyId, field: keyof Rate, event: Event) {
		const raw = (event.currentTarget as HTMLInputElement).value;
		const value = Number(raw);
		if (raw !== '' && Number.isFinite(value) && value >= 0) rates.set(family, field, value);
	}
</script>

<div class="flex flex-col gap-2">
	<div class="overflow-x-auto">
		<table class="w-full text-xs">
			<thead>
				<tr class="text-muted-foreground">
					<th class="py-1 pr-2 text-left font-medium">{t('cost.column.model')}</th>
					{#each FIELDS as field (field.id)}
						<th class="py-1 pr-2 text-right font-medium">{t(field.label)}</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#each FAMILIES as family (family.id)}
					<tr>
						<td class="py-1 pr-2 font-medium capitalize">{family.id}</td>
						{#each FIELDS as field (field.id)}
							<td class="py-1 pr-2">
								<input
									type="number"
									min="0"
									step="0.01"
									class="h-7 w-20 rounded border bg-transparent px-1.5 text-right text-xs tabular-nums"
									value={rates.for(family.id)[field.id]}
									aria-label={`${family.id} — ${t(field.label)}`}
									oninput={(event: Event) => onRate(family.id, field.id, event)}
								/>
							</td>
						{/each}
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	<div class="flex flex-wrap items-center gap-2">
		<span class="text-xs text-muted-foreground">{t('settings.rates.unit')}</span>
		{#if edited}
			<Badge variant="secondary" class="font-normal">{t('settings.rates.edited')}</Badge>
			<Button
				variant="ghost"
				size="sm"
				class="h-6 gap-1 px-2 text-xs font-normal"
				onclick={() => rates.reset()}
			>
				<RotateCcw class="size-3" />
				{t('settings.rates.reset')}
			</Button>
		{/if}
	</div>
</div>
