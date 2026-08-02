<script lang="ts">
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Separator } from '$lib/components/ui/separator';
	import { LOCALES, i18n, t, type LocaleSetting } from '$lib/i18n/index.svelte';
	import { PRESETS, theme, type BrandToken, type ThemeMode } from '$lib/theme.svelte';

	const MODES: ThemeMode[] = ['light', 'dark', 'system'];

	const TOKENS: { id: BrandToken; label: string; hint: string }[] = $derived([
		{ id: 'primary', label: t('settings.brand.primary'), hint: t('settings.brand.primaryHint') },
		{ id: 'accent', label: t('settings.brand.accent'), hint: t('settings.brand.accentHint') }
	]);

	const LANGUAGES: { id: LocaleSetting; label: string }[] = $derived([
		{ id: 'system', label: t('settings.language.system') },
		...LOCALES.map((entry) => ({ id: entry.id, label: entry.label }))
	]);

	const hasBrand = $derived(
		Object.values(theme.brand.light).some(Boolean) || Object.values(theme.brand.dark).some(Boolean)
	);

	function pick(themeMode: 'light' | 'dark', token: BrandToken, event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		theme.setBrandColor(themeMode, token, input.value);
	}
</script>

<div class="flex h-full flex-col gap-4 overflow-auto p-6">
	<Card.Root>
		<Card.Header>
			<Card.Title>{t('settings.language')}</Card.Title>
			<Card.Description>{t('settings.language.description')}</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-wrap items-center gap-2">
			{#each LANGUAGES as language (language.id)}
				<Button
					variant={i18n.setting === language.id ? 'default' : 'outline'}
					size="sm"
					onclick={() => i18n.set(language.id)}
				>
					{language.label}
				</Button>
			{/each}
		</Card.Content>
	</Card.Root>

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
			<div class="flex items-center justify-between">
				<div>
					<Card.Title>{t('settings.brand')}</Card.Title>
					<Card.Description>{t('settings.brand.description')}</Card.Description>
				</div>
				{#if hasBrand}
					<Button variant="outline" size="sm" onclick={() => theme.resetBrand()}>
						<RotateCcw class="size-3.5" />
						{t('settings.brand.reset')}
					</Button>
				{/if}
			</div>
		</Card.Header>
		<Card.Content class="flex flex-col gap-4">
			{#each TOKENS as token (token.id)}
				<div class="flex flex-wrap items-center gap-4">
					<div class="w-48">
						<p class="text-sm font-medium">{token.label}</p>
						<p class="text-xs text-muted-foreground">{token.hint}</p>
					</div>
					{#each ['light', 'dark'] as const as themeMode (themeMode)}
						<div class="flex items-center gap-2">
							<span class="w-14 text-xs text-muted-foreground">
								{t(`settings.mode.${themeMode}`)}
							</span>
							<input
								type="color"
								class="size-8 cursor-pointer rounded border bg-transparent"
								value={theme.brand[themeMode][token.id] ?? '#808080'}
								oninput={(event) => pick(themeMode, token.id, event)}
								aria-label={t('settings.brand.colourFor', {
									token: token.label,
									mode: t(`settings.mode.${themeMode}`)
								})}
							/>
							{#if theme.brand[themeMode][token.id]}
								<span class="font-mono text-xs">{theme.brand[themeMode][token.id]}</span>
								<Button
									variant="ghost"
									size="sm"
									class="h-6 px-1.5 text-xs"
									onclick={() => theme.setBrandColor(themeMode, token.id, null)}
								>
									{t('settings.brand.clear')}
								</Button>
							{:else}
								<span class="text-xs text-muted-foreground">{t('settings.brand.fromPreset')}</span>
							{/if}
						</div>
					{/each}
				</div>
			{/each}

			<Separator />

			<div class="flex items-center gap-3">
				<span class="text-xs text-muted-foreground">{t('settings.preview')}</span>
				<Button size="sm">{t('settings.preview.primary')}</Button>
				<Button variant="outline" size="sm">{t('settings.preview.outline')}</Button>
				<span class="rounded-md bg-accent px-2 py-1 text-xs text-accent-foreground">
					{t('settings.preview.accent')}
				</span>
				<Badge>{t('settings.preview.badge')}</Badge>
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>{t('settings.planned')}</Card.Title>
			<Card.Description>{t('settings.planned.description')}</Card.Description>
		</Card.Header>
	</Card.Root>
</div>
