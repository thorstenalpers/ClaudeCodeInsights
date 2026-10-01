<script lang="ts">
	/**
	 * What this app is, where it keeps things, and what is still missing.
	 *
	 * The links are placeholders on purpose: there is no repository and no
	 * documentation yet, and a page that quietly omitted them would hide that.
	 * They are listed with what they are waiting for instead.
	 */
	import { api, type AppInfo } from '$lib/api';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { displayPath } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { isHosted } from '$lib/ipc.svelte';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import Bug from '@lucide/svelte/icons/bug';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import Repository from '@lucide/svelte/icons/git-branch';
	import Info from '@lucide/svelte/icons/info';
	import Link from '@lucide/svelte/icons/link';
	import Scale from '@lucide/svelte/icons/scale';
	import Scroll from '@lucide/svelte/icons/scroll-text';

	let info = $state<AppInfo | null>(null);

	$effect(() => {
		if (!isHosted) return;
		void api.getAppInfo().then((value) => (info = value));
	});

	/** Everything the app writes, and where. */
	const places = $derived([
		{ label: t('info.place.data'), path: info?.dataDir, open: 'data' },
		{ label: t('info.place.database'), path: info?.database, open: null },
		{ label: t('info.place.voices'), path: info?.voices, open: 'voices' },
		{ label: t('info.place.logs'), path: info?.logs, open: null }
	]);

	/**
	 * The links a released app would have, and does not yet.
	 *
	 * Written down rather than left out: the addresses are the placeholders they
	 * look like, and naming them is how they stop being forgotten.
	 */
	const TODO = [
		{ label: 'info.link.repository', hint: 'info.link.repository.todo', icon: Repository },
		{ label: 'info.link.docs', hint: 'info.link.docs.todo', icon: BookOpen },
		{ label: 'info.link.issues', hint: 'info.link.issues.todo', icon: Bug },
		{ label: 'info.link.licenses', hint: 'info.link.licenses.todo', icon: Scale }
	] as const;
</script>

<div class="h-full overflow-y-auto">
	<div class="mx-auto flex max-w-2xl flex-col gap-4 p-5">
		<header>
			<h1 class="text-xl font-semibold tracking-tight">{t('info.title')}</h1>
			<p class="mt-0.5 text-xs text-muted-foreground">{t('info.subtitle')}</p>
		</header>

		<Card.Root>
			<Card.Header>
				<Card.Title class="flex items-center gap-2 text-base">
					<Info class="size-3.5 text-muted-foreground" />
					{t('app.name')}
					{#if info}
						<Badge variant="secondary" class="font-normal tabular-nums">{info.version}</Badge>
					{/if}
				</Card.Title>
				<Card.Description>{t('info.app.body')}</Card.Description>
			</Card.Header>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title class="flex items-center gap-2 text-base">
					<Scroll class="size-3.5 text-muted-foreground" />
					{t('info.places.title')}
				</Card.Title>
				<Card.Description>{t('info.places.description')}</Card.Description>
			</Card.Header>
			<Card.Content class="flex flex-col gap-2">
				{#if !isHosted}
					<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
				{:else if !info}
					<Skeleton class="h-20 w-full" />
				{:else}
					{#each places as place (place.label)}
						<div class="flex flex-wrap items-center gap-2 border-b pb-2 last:border-0 last:pb-0">
							<span class="w-24 shrink-0 text-xs text-muted-foreground">{place.label}</span>
							<span
								class="min-w-0 flex-1 truncate font-mono text-xs break-all"
								title={place.path ?? ''}
							>
								{place.path ? displayPath(place.path) : t('common.none')}
							</span>
							{#if place.open}
								<Button
									variant="ghost"
									size="sm"
									class="h-7 shrink-0 font-normal"
									onclick={() => void api.openDataFolder(place.open)}
								>
									<FolderOpen class="size-3.5" />
									{t('info.place.open')}
								</Button>
							{/if}
						</div>
					{/each}
				{/if}
			</Card.Content>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title class="flex items-center gap-2 text-base">
					<Link class="size-3.5 text-muted-foreground" />
					{t('info.links.title')}
				</Card.Title>
				<Card.Description>{t('info.links.description')}</Card.Description>
			</Card.Header>
			<Card.Content class="flex flex-col gap-2">
				{#each TODO as entry (entry.label)}
					{@const Icon = entry.icon}
					<div class="flex flex-wrap items-center gap-2 border-b pb-2 last:border-0 last:pb-0">
						<Icon class="size-3.5 shrink-0 text-muted-foreground" />
						<span class="text-sm">{t(entry.label)}</span>
						<Badge variant="outline" class="font-normal">TODO</Badge>
						<span class="w-full text-xs text-muted-foreground @md:w-auto @md:flex-1">
							{t(entry.hint)}
						</span>
					</div>
				{/each}
			</Card.Content>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title class="flex items-center gap-2 text-base">
					<Scale class="size-3.5 text-muted-foreground" />
					{t('info.privacy.title')}
				</Card.Title>
			</Card.Header>
			<Card.Content>
				<p class="text-xs leading-relaxed text-muted-foreground">{t('info.privacy.body')}</p>
			</Card.Content>
		</Card.Root>
	</div>
</div>
