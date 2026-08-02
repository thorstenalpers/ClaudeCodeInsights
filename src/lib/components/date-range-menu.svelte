<script lang="ts">
	import CalendarIcon from '@lucide/svelte/icons/calendar';
	import Check from '@lucide/svelte/icons/check';
	import { Button } from '$lib/components/ui/button';
	import { RangeCalendar } from '$lib/components/ui/range-calendar';
	import * as Popover from '$lib/components/ui/popover';
	import { t } from '$lib/i18n/index.svelte';
	import { CalendarDate, getLocalTimeZone, today } from '@internationalized/date';
	import type { DateRange } from 'bits-ui';

	type Props = {
		from: string | null;
		to: string | null;
		onChange: (from: string | null, to: string | null) => void;
	};

	let { from, to, onChange }: Props = $props();

	// The window always ends today; only how far back it reaches is a choice.
	// That is what "the last seven days" means, and it keeps the presets honest
	// when the app is left open across midnight.
	const PRESETS = [
		{ days: 0, key: 'range.all' as const },
		{ days: 7, key: 'range.7' as const },
		{ days: 14, key: 'range.14' as const },
		{ days: 30, key: 'range.30' as const },
		{ days: 90, key: 'range.90' as const }
	];

	function iso(date: CalendarDate): string {
		return date.toString();
	}

	function applyPreset(days: number) {
		if (days === 0) {
			onChange(null, null);
			return;
		}
		const end = today(getLocalTimeZone());
		onChange(iso(end.subtract({ days: days - 1 })), iso(end));
	}

	function parse(value: string | null): CalendarDate | undefined {
		if (!value) return undefined;
		const [year, month, day] = value.split('-').map(Number);
		return year && month && day ? new CalendarDate(year, month, day) : undefined;
	}

	const value = $derived<DateRange>({ start: parse(from), end: parse(to) });

	const activeDays = $derived.by(() => {
		if (!from || !to) return 0;
		const end = today(getLocalTimeZone());
		if (to !== iso(end)) return -1;
		const start = parse(from);
		if (!start) return -1;
		const match = PRESETS.find(
			(preset) => preset.days > 0 && iso(end.subtract({ days: preset.days - 1 })) === from
		);
		return match?.days ?? -1;
	});

	const label = $derived.by(() => {
		if (!from && !to) return t('range.all');
		const preset = PRESETS.find((entry) => entry.days === activeDays);
		return preset ? t(preset.key) : `${from ?? '…'} – ${to ?? '…'}`;
	});
</script>

<Popover.Root>
	<Popover.Trigger>
		{#snippet child({ props })}
			<Button
				{...props}
				variant={from || to ? 'default' : 'outline'}
				size="sm"
				class="h-8 gap-1 font-normal"
			>
				<CalendarIcon class="size-3.5" />
				{label}
			</Button>
		{/snippet}
	</Popover.Trigger>

	<Popover.Content class="w-auto p-0" align="start">
		<div class="flex">
			<div class="flex flex-col gap-1 border-r p-2">
				{#each PRESETS as preset (preset.days)}
					<Button
						variant="ghost"
						size="sm"
						class="justify-start gap-2 font-normal"
						onclick={() => applyPreset(preset.days)}
					>
						{#if activeDays === preset.days}
							<Check class="size-3.5" />
						{:else}
							<span class="size-3.5"></span>
						{/if}
						{t(preset.key)}
					</Button>
				{/each}
			</div>

			<RangeCalendar
				{value}
				onValueChange={(next: DateRange | undefined) => {
					onChange(next?.start?.toString() ?? null, next?.end?.toString() ?? null);
				}}
			/>
		</div>
	</Popover.Content>
</Popover.Root>
