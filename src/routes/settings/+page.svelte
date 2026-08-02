<script lang="ts">
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Separator } from '$lib/components/ui/separator';
	import { PRESETS, theme, type BrandToken, type ThemeMode } from '$lib/theme.svelte';

	const MODES: { id: ThemeMode; label: string }[] = [
		{ id: 'light', label: 'Light' },
		{ id: 'dark', label: 'Dark' },
		{ id: 'system', label: 'System' }
	];

	const TOKENS: { id: BrandToken; label: string; hint: string }[] = [
		{ id: 'primary', label: 'Primary', hint: 'Buttons, active navigation, focus ring' },
		{ id: 'accent', label: 'Accent', hint: 'Hover surfaces, subtle highlights' }
	];

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
			<Card.Title>Appearance</Card.Title>
			<Card.Description>Mode and colour preset. Both survive a restart.</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-col gap-4">
			<div class="flex items-center gap-2">
				<span class="w-24 text-sm text-muted-foreground">Mode</span>
				{#each MODES as m (m.id)}
					<Button
						variant={theme.mode === m.id ? 'default' : 'outline'}
						size="sm"
						onclick={() => theme.setMode(m.id)}
					>
						{m.label}
					</Button>
				{/each}
			</div>
			<div class="flex items-center gap-2">
				<span class="w-24 text-sm text-muted-foreground">Preset</span>
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
					<Card.Title>Brand colours</Card.Title>
					<Card.Description>
						Override the preset's key colours, separately for light and dark mode. Empty means the
						preset decides.
					</Card.Description>
				</div>
				{#if hasBrand}
					<Button variant="outline" size="sm" onclick={() => theme.resetBrand()}>
						<RotateCcw class="size-3.5" />
						Reset
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
							<span class="w-10 text-xs text-muted-foreground capitalize">{themeMode}</span>
							<input
								type="color"
								class="size-8 cursor-pointer rounded border bg-transparent"
								value={theme.brand[themeMode][token.id] ?? '#808080'}
								oninput={(event) => pick(themeMode, token.id, event)}
								aria-label={`${token.label} colour for ${themeMode} mode`}
							/>
							{#if theme.brand[themeMode][token.id]}
								<span class="font-mono text-xs">{theme.brand[themeMode][token.id]}</span>
								<Button
									variant="ghost"
									size="sm"
									class="h-6 px-1.5 text-xs"
									onclick={() => theme.setBrandColor(themeMode, token.id, null)}
								>
									Clear
								</Button>
							{:else}
								<span class="text-xs text-muted-foreground">preset</span>
							{/if}
						</div>
					{/each}
				</div>
			{/each}

			<Separator />

			<div class="flex items-center gap-3">
				<span class="text-xs text-muted-foreground">Preview</span>
				<Button size="sm">Primary action</Button>
				<Button variant="outline" size="sm">Outline</Button>
				<span class="rounded-md bg-accent px-2 py-1 text-xs text-accent-foreground">
					Accent surface
				</span>
				<Badge>Badge</Badge>
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Planned</Card.Title>
			<Card.Description>
				Transcript folders to scan · API key per provider, stored in the OS keychain
			</Card.Description>
		</Card.Header>
	</Card.Root>
</div>
