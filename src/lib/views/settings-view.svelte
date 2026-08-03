<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import Download from '@lucide/svelte/icons/download';
	import Volume2 from '@lucide/svelte/icons/volume-2';
	import Square from '@lucide/svelte/icons/square';
	import Mic from '@lucide/svelte/icons/mic';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api';
	import { displayPath } from '$lib/format';
	import { LOCALES, i18n, t, type LocaleSetting } from '$lib/i18n/index.svelte';
	import { SUBSCRIPTIONS, billing, plan, type BillingMode, type PlanId } from '$lib/pricing.svelte';
	import ApiKeys from '$lib/components/api-keys.svelte';
	import RateTable from '$lib/components/rate-table.svelte';
	import { cli } from '$lib/cli.svelte';
	import { logs } from '$lib/logs.svelte';
	import { PRESETS, theme, type ThemeMode } from '$lib/theme.svelte';
	import { language, packChoice, voice } from '$lib/voice.svelte';

	const MODES: ThemeMode[] = ['light', 'dark', 'system'];

	let hubQuery = $state('');

	$effect(() => {
		void voice.check();
		void voice.loadPacks();
		void voice.loadMicrophones();
	});
	const BILLING: BillingMode[] = ['api', 'subscription'];

	const LANGUAGES: { id: LocaleSetting; label: string }[] = $derived([
		{ id: 'system', label: t('settings.language.system') },
		...LOCALES.map((entry) => ({ id: entry.id, label: entry.label }))
	]);

	function onCliInput(event: Event) {
		cli.set((event.currentTarget as HTMLInputElement).value);
	}

	function onPlanPrice(id: PlanId, event: Event) {
		const raw = (event.currentTarget as HTMLInputElement).value;
		const value = Number(raw);
		if (raw !== '' && Number.isFinite(value) && value >= 0) plan.setPrice(id, value);
	}

	const pricesEdited = $derived(SUBSCRIPTIONS.some((entry) => plan.isPriced(entry.id)));

	// The app's own language first: that is the one the answers are written in,
	// and the reason a voice is being picked at all.
	const spoken = $derived(voice.forLanguage(i18n.intlLocale));
	const voiceOptions = $derived([
		...spoken,
		...voice.voices.filter((entry) => !spoken.includes(entry))
	]);
	// Every speaker of an installed pack, as one flat list beside the Windows
	// voices — a reader picking a voice does not care which engine speaks it.
	const packSpeakers = $derived(
		voice.packs
			.filter((pack) => pack.installed)
			.flatMap((pack) =>
				Array.from({ length: pack.voices }, (_, index) => ({
					value: packChoice(pack.id, index),
					// The number only helps where there are several to tell apart.
					label: pack.voices > 1 ? `${pack.label} ${index + 1}` : pack.label,
					language: pack.language
				}))
			)
	);

	const currentVoice = $derived(
		voice.voices.find((entry) => entry.name === voice.name)?.name ??
			packSpeakers.find((entry) => entry.value === voice.name)?.label ??
			t('settings.voice.voice.auto')
	);

	// The sample, like every answer, is written in the language the app is set
	// to — never in the pack's. A pack that does not speak that language reads
	// it as if it were its own, which sounds like a fault in the voice rather
	// than a mismatched pair, so it is said here instead.
	const wrongLanguage = $derived.by(() => {
		const chosen = packSpeakers.find((entry) => entry.value === voice.name);
		if (!chosen) return null;
		const wanted = language(i18n.intlLocale);
		return chosen.language.split(',').some((tag) => language(tag) === wanted) ? null : wanted;
	});

	const currentLanguage = $derived(
		LANGUAGES.find((entry) => entry.id === i18n.setting)?.label ?? t('settings.language.system')
	);
</script>

<div class="flex h-full flex-col gap-3 overflow-auto p-4">
	<Card.Root data-size="sm" class="shrink-0">
		<Card.Header>
			<Card.Title>{t('settings.appearance')}</Card.Title>
			<Card.Description>{t('settings.appearance.description')}</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-col gap-3">
			<div class="flex items-center gap-2">
				<span class="w-24 shrink-0 text-xs text-muted-foreground">{t('settings.mode')}</span>
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
				<span class="w-24 shrink-0 text-xs text-muted-foreground">{t('settings.preset')}</span>
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

	<Card.Root data-size="sm" class="shrink-0">
		<Card.Header>
			<Card.Title>{t('settings.language')}</Card.Title>
			<Card.Description>{t('settings.language.description')}</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-col gap-3">
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

			<div class="flex flex-col gap-1 border-t pt-3">
				<span class="text-xs text-muted-foreground">{t('settings.voice.voice')}</span>
				<div class="flex flex-wrap items-center gap-2">
					<DropdownMenu.Root>
						<DropdownMenu.Trigger>
							{#snippet child({ props })}
								<Button {...props} variant="outline" class="w-64 justify-between">
									<span class="truncate">{currentVoice}</span>
									<ChevronDown class="size-4 shrink-0 opacity-60" />
								</Button>
							{/snippet}
						</DropdownMenu.Trigger>
						<DropdownMenu.Content class="max-h-80 w-64 overflow-y-auto">
							<DropdownMenu.Item onSelect={() => voice.setName('')}>
								<span class="flex-1">{t('settings.voice.voice.auto')}</span>
								{#if voice.name === ''}
									<Check class="size-4" />
								{/if}
							</DropdownMenu.Item>
							{#if packSpeakers.length > 0}
								<DropdownMenu.Separator />
								{#each packSpeakers as option (option.value)}
									<DropdownMenu.Item onSelect={() => voice.setName(option.value)}>
										<span class="flex flex-1 flex-col">
											<span class="truncate">{option.label}</span>
											<span class="text-xs text-muted-foreground">{option.language}</span>
										</span>
										{#if voice.name === option.value}
											<Check class="size-4" />
										{/if}
									</DropdownMenu.Item>
								{/each}
							{/if}
							<DropdownMenu.Separator />
							{#each voiceOptions as option (option.name)}
								<DropdownMenu.Item onSelect={() => voice.setName(option.name)}>
									<span class="flex flex-1 flex-col">
										<span class="truncate">{option.name}</span>
										<span class="text-xs text-muted-foreground">{option.lang}</span>
									</span>
									{#if voice.name === option.name}
										<Check class="size-4" />
									{/if}
								</DropdownMenu.Item>
							{/each}
						</DropdownMenu.Content>
					</DropdownMenu.Root>

					{#if voice.speaking}
						<Button
							variant="ghost"
							size="sm"
							class="h-8 gap-1 font-normal"
							onclick={() => voice.silence()}
						>
							<Square class="size-4" />
							{t('voice.stop')}
						</Button>
					{:else}
						<Button
							variant="ghost"
							size="sm"
							class="h-8 gap-1 font-normal"
							onclick={() => voice.speak(t('settings.voice.sample'))}
						>
							<Volume2 class="size-4" />
							{t('settings.voice.test')}
						</Button>
					{/if}
				</div>

				{#if wrongLanguage}
					<p class="max-w-md text-xs text-amber-600 dark:text-amber-500">
						{t('settings.voice.mismatch', { language: wrongLanguage })}
					</p>
				{/if}

				{#if spoken.length === 0 && packSpeakers.length === 0}
					<p class="max-w-md text-xs text-muted-foreground">{t('settings.voice.noVoice')}</p>
				{/if}
			</div>

			<div class="flex flex-col gap-2 border-t pt-3">
				<span class="text-xs text-muted-foreground">{t('settings.voice.packs')}</span>
				{#each voice.packs as pack (pack.id)}
					<div class="flex flex-wrap items-center gap-2">
						<span class="flex-1 text-sm">
							{pack.label}
							<span class="text-xs text-muted-foreground">
								{pack.installed
									? t('settings.voice.packs.installed', { count: pack.voices })
									: t('settings.voice.packs.size', { size: pack.megabytes })}
							</span>
						</span>
						{#if pack.installed}
							<Button
								variant="ghost"
								size="sm"
								class="h-8 font-normal"
								onclick={() => void voice.remove(pack.id)}
							>
								{t('settings.voice.packs.remove')}
							</Button>
						{:else if voice.isInstalling(pack.id)}
							<!-- A bar and a number, because "Downloading…" for six minutes
							     is indistinguishable from a download that has died. -->
							<div class="flex min-w-40 items-center gap-2">
								<div class="h-1.5 min-w-16 flex-1 overflow-hidden rounded-full bg-muted">
									<div
										class="h-full rounded-full bg-primary transition-[width] duration-300"
										style:width={voice.percentOf(pack.id) === null
											? '100%'
											: `${voice.percentOf(pack.id)}%`}
									></div>
								</div>
								<span class="w-10 shrink-0 text-right text-xs text-muted-foreground tabular-nums">
									{voice.percentOf(pack.id) === null ? '' : `${voice.percentOf(pack.id)}%`}
								</span>
								<Button
									variant="ghost"
									size="sm"
									class="h-8 font-normal"
									onclick={() => void voice.cancel(pack.id)}
								>
									{t('settings.voice.packs.cancel')}
								</Button>
							</div>
						{:else}
							<Button
								variant="outline"
								size="sm"
								class="h-8 gap-1 font-normal"
								onclick={() => void voice.install(pack.id)}
							>
								<Download class="size-3.5" />
								{t('settings.voice.packs.install')}
							</Button>
						{/if}
					</div>
				{/each}
				<p class="max-w-md text-xs text-muted-foreground">{t('settings.voice.packs.hint')}</p>
				{#if voice.folder}
					<p class="max-w-md text-xs break-all text-muted-foreground">
						{t('settings.voice.packs.folder', { path: displayPath(voice.folder) })}
					</p>
				{/if}
				{#if voice.error}
					<p class="text-xs text-destructive">{voice.error}</p>
				{/if}
			</div>

			<div class="flex flex-col gap-2 border-t pt-3">
				<span class="text-xs text-muted-foreground">{t('settings.voice.hub')}</span>
				<form
					class="flex flex-wrap items-center gap-2"
					onsubmit={(event) => {
						event.preventDefault();
						void voice.searchHub(hubQuery);
					}}
				>
					<Input
						class="h-8 max-w-64 flex-1"
						bind:value={hubQuery}
						placeholder={t('settings.voice.hub.placeholder')}
					/>
					<Button
						type="submit"
						variant="outline"
						size="sm"
						class="h-8 font-normal"
						disabled={voice.hubSearching || !hubQuery.trim()}
					>
						{t('settings.voice.hub.search')}
					</Button>
				</form>

				{#each voice.hubResults as model (model.repo)}
					<div class="flex flex-wrap items-center gap-2">
						<span class="flex-1 text-sm break-all">
							{model.repo}
							<span class="text-xs text-muted-foreground">
								{model.languages.join(', ')}
								{model.megabytes > 0
									? ` · ${t('settings.voice.packs.size', { size: model.megabytes })}`
									: ''}
							</span>
						</span>
						{#if model.blocked}
							<!-- Named rather than hidden: a reader who came with a link
							     deserves to know why it is not on offer. -->
							<span class="text-xs text-muted-foreground">
								{t('settings.voice.hub.blocked', { reason: model.blocked })}
							</span>
						{:else if voice.isInstalling(`hub:${model.repo}`)}
							<span class="text-xs text-muted-foreground tabular-nums">
								{voice.percentOf(`hub:${model.repo}`) === null
									? ''
									: `${voice.percentOf(`hub:${model.repo}`)}%`}
							</span>
							<Button
								variant="ghost"
								size="sm"
								class="h-8 font-normal"
								onclick={() => void voice.cancel(`hub:${model.repo}`)}
							>
								{t('settings.voice.packs.cancel')}
							</Button>
						{:else}
							<Button
								variant="outline"
								size="sm"
								class="h-8 gap-1 font-normal"
								onclick={() => void voice.install(`hub:${model.repo}`, model.repo, model.megabytes)}
							>
								<Download class="size-3.5" />
								{t('settings.voice.packs.install')}
							</Button>
						{/if}
					</div>
				{/each}

				{#if voice.hubNote}
					<p class="max-w-md text-xs text-muted-foreground">{voice.hubNote}</p>
				{/if}
				<p class="max-w-md text-xs text-muted-foreground">{t('settings.voice.hub.hint')}</p>
			</div>

			<div class="flex flex-col gap-2 border-t pt-3">
				<span class="text-xs text-muted-foreground">{t('settings.voice.mic')}</span>
				<p class="max-w-md text-xs text-muted-foreground">{t('settings.voice.mic.hint')}</p>
				{#each voice.microphones as microphone (microphone.name)}
					<div class="flex flex-wrap items-center gap-2">
						<Mic class="size-3.5 shrink-0 text-muted-foreground" />
						<span class="min-w-0 flex-1 truncate text-sm" title={microphone.name}>
							{microphone.name}
						</span>
						{#if microphone.isDefault}
							<Badge variant="secondary" class="font-normal">
								{t('settings.voice.mic.default')}
							</Badge>
						{/if}
					</div>
				{/each}
				{#if voice.microphones.length === 0}
					<p class="max-w-md text-xs text-muted-foreground">{t('settings.voice.mic.none')}</p>
				{/if}
				<div>
					<Button
						variant="outline"
						size="sm"
						class="h-8 font-normal"
						onclick={() => void api.openSoundSettings()}
					>
						{t('settings.voice.mic.open')}
					</Button>
				</div>
			</div>

			<div class="flex flex-col gap-2 border-t pt-3">
				<span class="text-xs text-muted-foreground">{t('settings.voice.windows')}</span>
				<p class="max-w-md text-xs text-muted-foreground">{t('settings.voice.windows.hint')}</p>
				<div>
					<Button
						variant="outline"
						size="sm"
						class="h-8 font-normal"
						onclick={() => void api.openSpeechSettings()}
					>
						{t('settings.voice.windows.open')}
					</Button>
				</div>
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root data-size="sm" class="shrink-0">
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
			{#if cli.status}
				<p class="text-xs text-muted-foreground">
					{cli.status.found && cli.status.path
						? t('settings.cli.found', { path: displayPath(cli.status.path) })
						: t('settings.cli.missing')}
				</p>
			{/if}
		</Card.Content>
	</Card.Root>

	<Card.Root data-size="sm" class="shrink-0">
		<Card.Header>
			<Card.Title>{t('settings.keys')}</Card.Title>
			<Card.Description>{t('settings.keys.description')}</Card.Description>
		</Card.Header>
		<Card.Content>
			<ApiKeys />
		</Card.Content>
	</Card.Root>

	<Card.Root data-size="sm" class="shrink-0">
		<Card.Header>
			<Card.Title>{t('settings.billing')}</Card.Title>
			<Card.Description>{t('settings.billing.description')}</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-col gap-2">
			{#each BILLING as option (option)}
				<button
					type="button"
					class="flex items-start gap-2 rounded-md border p-2 text-left transition-colors hover:bg-primary/10"
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

			<!-- Only the paid plans: on the free one there is no subscription to
			     weigh, and nothing to correct the price of. -->
			{#if billing.mode === 'subscription'}
				<div class="mt-1 flex flex-col gap-2 border-t pt-3">
					<span class="text-xs text-muted-foreground">{t('cost.plan')}</span>
					{#each SUBSCRIPTIONS as entry (entry.id)}
						<div class="flex items-center gap-2">
							<button
								type="button"
								class="flex flex-1 items-center gap-2 rounded-md border p-2 text-left transition-colors hover:bg-primary/10"
								class:border-primary={plan.id === entry.id}
								onclick={() => plan.set(entry.id)}
							>
								<span
									class="size-4 shrink-0 rounded-full border-2"
									class:bg-primary={plan.id === entry.id}
									class:border-primary={plan.id === entry.id}
								></span>
								<span class="text-sm font-medium">{t(`cost.plan.${entry.id}`)}</span>
							</button>
							<Input
								type="number"
								step="1"
								min="0"
								class="h-8 w-24 text-right tabular-nums"
								aria-label={`${t(`cost.plan.${entry.id}`)} — ${t('settings.plan.unit')}`}
								value={plan.monthlyOf(entry.id)}
								oninput={(event: Event) => onPlanPrice(entry.id, event)}
							/>
						</div>
					{/each}

					<div class="flex flex-wrap items-center gap-2">
						<span class="text-xs text-muted-foreground">{t('settings.plan.unit')}</span>
						{#if pricesEdited}
							<Badge variant="secondary" class="font-normal">{t('settings.rates.edited')}</Badge>
							<Button
								variant="ghost"
								size="sm"
								class="h-6 gap-1 px-2 text-xs font-normal"
								onclick={() => plan.resetPrices()}
							>
								<RotateCcw class="size-3" />
								{t('settings.rates.reset')}
							</Button>
						{/if}
					</div>
				</div>
			{/if}
		</Card.Content>
	</Card.Root>

	<Card.Root data-size="sm" class="shrink-0">
		<Card.Header>
			<Card.Title>{t('settings.rates')}</Card.Title>
			<Card.Description>{t('settings.rates.description')}</Card.Description>
		</Card.Header>
		<Card.Content>
			<RateTable />
		</Card.Content>
	</Card.Root>

	<Card.Root data-size="sm" class="shrink-0">
		<Card.Header>
			<Card.Title>{t('settings.logs')}</Card.Title>
			<Card.Description>{t('settings.logs.description')}</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-wrap items-center gap-2">
			<Button
				variant={logs.enabled ? 'default' : 'outline'}
				size="sm"
				onclick={() => logs.setEnabled(!logs.enabled)}
			>
				{logs.enabled ? t('settings.logs.on') : t('settings.logs.off')}
			</Button>
			{#if logs.enabled}
				<Button variant="outline" size="sm" href={resolve('/logs')}
					>{t('settings.logs.open')}</Button
				>
			{/if}
		</Card.Content>
	</Card.Root>
</div>
