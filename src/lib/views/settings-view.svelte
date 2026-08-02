<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { LOCALES, i18n, t, type LocaleSetting } from '$lib/i18n/index.svelte';
	import { billing, type BillingMode } from '$lib/pricing.svelte';
	import { api, type CliStatus } from '$lib/api';
	import { cli } from '$lib/cli.svelte';
	import { isHosted } from '$lib/ipc.svelte';
	import { CURRENCIES, region, type CurrencySetting } from '$lib/region.svelte';
	import { PRESETS, theme, type ThemeMode } from '$lib/theme.svelte';

	const MODES: ThemeMode[] = ['light', 'dark', 'system'];
	const BILLING: BillingMode[] = ['api', 'subscription'];

	const LANGUAGES: { id: LocaleSetting; label: string }[] = $derived([
		{ id: 'system', label: t('settings.language.system') },
		...LOCALES.map((entry) => ({ id: entry.id, label: entry.label }))
	]);

	const CURRENCY_OPTIONS: { id: CurrencySetting; label: string }[] = $derived([
		{ id: 'system', label: t('settings.region.system') },
		...CURRENCIES.map((entry) => ({ id: entry, label: entry }))
	]);

	const currentCurrency = $derived(
		CURRENCY_OPTIONS.find((entry) => entry.id === region.setting)?.label ??
			t('settings.region.system')
	);

	let cliStatus = $state<CliStatus | null>(null);

	$effect(() => {
		void cli.path;
		if (isHosted) void api.getCliStatus(cli.configured).then((value) => (cliStatus = value));
	});

	function onCliInput(event: Event) {
		cli.set((event.currentTarget as HTMLInputElement).value);
	}

	function onRateInput(event: Event) {
		const raw = (event.currentTarget as HTMLInputElement).value;
		region.setRate(raw === '' ? null : Number(raw));
	}

	const currentLanguage = $derived(
		LANGUAGES.find((entry) => entry.id === i18n.setting)?.label ?? t('settings.language.system')
	);
</script>

<div class="flex h-full flex-col gap-4 overflow-auto p-6">
	<Card.Root>
		<Card.Header>
			<Card.Title>{t('settings.appearance')}</Card.Title>
			<Card.Description>{t('settings.appearance.description')}</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-col gap-4">
			<div class="flex items-center gap-2">
				<span class="w-24 text-sm text-muted-foreground">{t('settings.mode')}</span>
				{#each MODES as m (m)}
					<Button
						variant={theme.mode === m ? 'default' : 'outline'}
						size="sm"
						onclick={() => theme.setMode(m)}
					>
						{t(`settings.mode.${m}`)}
					</Button>
				{/each}
			</div>
			<div class="flex flex-wrap items-center gap-2">
				<span class="w-24 text-sm text-muted-foreground">{t('settings.preset')}</span>
				{#each PRESETS as preset (preset.id)}
					<Button
						variant={theme.preset === preset.id ? 'default' : 'outline'}
						size="sm"
						onclick={() => theme.setPreset(preset.id)}
					>
						{preset.label}
					</Button>
				{/each}
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>{t('settings.language')}</Card.Title>
			<Card.Description>{t('settings.language.description')}</Card.Description>
		</Card.Header>
		<Card.Content>
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="outline" class="w-56 justify-between">
							{currentLanguage}
							<ChevronDown class="size-4 opacity-60" />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content class="max-h-80 w-56 overflow-y-auto">
					{#each LANGUAGES as language (language.id)}
						<DropdownMenu.Item onSelect={() => i18n.set(language.id)}>
							<span class="flex-1">{language.label}</span>
							{#if i18n.setting === language.id}
								<Check class="size-4" />
							{/if}
						</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>{t('settings.region')}</Card.Title>
			<Card.Description>{t('settings.region.description')}</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-wrap items-end gap-4">
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="outline" class="w-56 justify-between">
							{currentCurrency}
							<ChevronDown class="size-4 opacity-60" />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content class="max-h-80 w-56 overflow-y-auto">
					{#each CURRENCY_OPTIONS as option (option.id)}
						<DropdownMenu.Item onSelect={() => region.set(option.id)}>
							<span class="flex-1">{option.label}</span>
							{#if region.setting === option.id}
								<Check class="size-4" />
							{/if}
						</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Content>
			</DropdownMenu.Root>

			{#if region.currency !== 'USD'}
				<div class="flex flex-col gap-1">
					<span class="text-xs text-muted-foreground">
						{t('settings.rate')} · {t('settings.rate.hint', { currency: region.currency })}
					</span>
					<Input
						type="number"
						step="0.01"
						min="0"
						class="w-32"
						value={region.rates[region.currency] ?? ''}
						oninput={onRateInput}
					/>
				</div>
			{/if}
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>{t('settings.cli')}</Card.Title>
			<Card.Description>{t('settings.cli.description')}</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-col gap-2">
			<Input
				class="max-w-lg font-mono text-xs"
				placeholder={t('settings.cli.placeholder')}
				value={cli.path}
				oninput={onCliInput}
			/>
			{#if cliStatus}
				<p class="text-xs text-muted-foreground">
					{cliStatus.found && cliStatus.path
						? t('settings.cli.found', { path: cliStatus.path })
						: t('settings.cli.missing')}
				</p>
			{/if}
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>{t('settings.billing')}</Card.Title>
			<Card.Description>{t('settings.billing.description')}</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-col gap-3">
			{#each BILLING as option (option)}
				<button
					type="button"
					class="flex items-start gap-3 rounded-md border p-3 text-left transition-colors hover:bg-accent"
					class:border-primary={billing.mode === option}
					onclick={() => billing.set(option)}
				>
					<span
						class="mt-0.5 size-4 shrink-0 rounded-full border-2"
						class:bg-primary={billing.mode === option}
						class:border-primary={billing.mode === option}
					></span>
					<span class="flex flex-col gap-0.5">
						<span class="text-sm font-medium">{t(`settings.billing.${option}`)}</span>
						<span class="text-xs text-muted-foreground">{t(`settings.billing.${option}Hint`)}</span>
					</span>
				</button>
			{/each}
		</Card.Content>
	</Card.Root>
</div>
