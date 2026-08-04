<script lang="ts">
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { resolve } from '$app/paths';
	import RailToggle from '$lib/components/rail-toggle.svelte';
	import { t } from '$lib/i18n/index.svelte';
	import { cn } from '$lib/utils';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import type { PageDefinition, PageGroup, SubEntry } from '$lib/pages';

	type Props = {
		groups: readonly PageGroup[];
		/** Pinned at the foot, in the order given. */
		footer: PageDefinition[];
		active: string;
		/** Shorter rows, no gaps and no group names, for a window too low to
		 *  hold the full set. */
		compact?: boolean;
		/**
		 * What hangs under an entry, by that entry's address.
		 *
		 * The rail is given them rather than reading them itself: they are the
		 * projects, activities, models, sessions and tools this machine has, and
		 * that is the pages' business, not the navigation's.
		 */
		subs?: Record<string, SubEntry[]>;
		/** The address on screen, which is what a sub-entry matches against. */
		path?: string;
	};

	let { groups, footer, active, compact = false, subs = {}, path = '' }: Props = $props();

	/** Which entries are folded open; an entry not named here is closed. */
	let open = $state<string[]>([]);

	function fold(href: string): void {
		open = open.includes(href) ? open.filter((entry) => entry !== href) : [...open, href];
	}

	/**
	 * What a menu entry does under the pointer.
	 *
	 * A background alone is flat, so the row leans a hair towards the page it
	 * would open and the icon grows with it. The bar on the left marks the page
	 * you are on and slides in rather than appearing, which is the difference
	 * between a state and a flicker.
	 */
	const HOVER = [
		'group/nav relative overflow-hidden transition-[background-color,color,transform] duration-150',
		'hover:translate-x-0.5 [&>svg]:transition-transform [&>svg]:duration-150 hover:[&>svg]:scale-110',
		'before:absolute before:top-1/2 before:left-0 before:h-5 before:w-1 before:-translate-x-1.5',
		'before:-translate-y-1/2 before:rounded-full before:bg-sidebar-primary before:transition-all',
		'before:duration-200 data-[active=true]:before:translate-x-0',
		'data-[active=true]:bg-sidebar-primary/15 data-[active=true]:text-sidebar-primary',
		'data-[active=true]:font-semibold data-[active=true]:hover:bg-sidebar-primary/20',
		'data-[active=true]:[&>svg]:text-sidebar-primary'
	].join(' ');
</script>

{#snippet entry(page: PageDefinition)}
	<Sidebar.MenuItem>
		<Sidebar.MenuButton size={compact ? 'sm' : 'default'} isActive={active === page.href}>
			{#snippet child({ props })}
				<a
					{...props}
					href={resolve(page.href)}
					class={cn((props as { class?: string }).class, HOVER)}
				>
					<page.icon />
					<span>{t(page.label)}</span>
				</a>
			{/snippet}
		</Sidebar.MenuButton>

		{@const children = subs[page.href] ?? []}
		{#if children.length > 0}
			{@const shown = open.includes(page.href) || path.startsWith(`${page.href}/`)}
			<!-- Folded away rather than gone: the arrow is the only thing that
			     stays when the views are hidden, so there is a way back to them. -->
			<Sidebar.MenuAction onclick={() => fold(page.href)} aria-expanded={shown}>
				<ChevronRight class={['transition-transform duration-150', shown && 'rotate-90']} />
				<span class="sr-only">{t('nav.subPages')}</span>
			</Sidebar.MenuAction>

			{#if shown}
				<Sidebar.MenuSub class={compact ? 'gap-0' : ''}>
					{#each children as item (item.href)}
						<Sidebar.MenuSubItem>
							<Sidebar.MenuSubButton
								size={compact ? 'sm' : 'md'}
								isActive={item.href === path}
								href={item.href}
								title={item.title ?? item.label}
							>
								<span class="truncate">{item.label}</span>
							</Sidebar.MenuSubButton>
						</Sidebar.MenuSubItem>
					{/each}
				</Sidebar.MenuSub>
			{/if}
		{/if}
	</Sidebar.MenuItem>
{/snippet}

<Sidebar.Root collapsible="icon">
	<Sidebar.Header>
		<div class="flex h-8 items-center gap-1 px-1">
			<!-- Above the menu it folds, left of the name: folded down to the rail it
			     is the only thing left up here, which is where the way out belongs. -->
			<RailToggle />
			<span
				class="truncate text-sm font-semibold tracking-tight group-data-[collapsible=icon]:hidden"
			>
				{t('app.name')}
			</span>
		</div>
	</Sidebar.Header>

	<!-- Scrolls folded down to the rail as well: the stock sidebar hides the
	     overflow there, so on a low window the last icons were unreachable
	     rather than merely out of sight. Downwards only — the rows are wider
	     than the rail, and a free horizontal axis put an empty column and a
	     bar under them. -->
	<Sidebar.Content
		class="group-data-[collapsible=icon]:overflow-x-hidden group-data-[collapsible=icon]:overflow-y-auto"
	>
		{#each groups as group (group.label)}
			<Sidebar.Group class={compact ? 'py-0' : ''}>
				{#if !compact}
					<Sidebar.GroupLabel>{t(group.label)}</Sidebar.GroupLabel>
				{/if}
				<Sidebar.GroupContent>
					<Sidebar.Menu class={compact ? 'gap-0' : 'gap-1'}>
						{#each group.pages as page (page.href)}
							{@render entry(page)}
						{/each}
					</Sidebar.Menu>
				</Sidebar.GroupContent>
			</Sidebar.Group>
		{/each}
	</Sidebar.Content>

	<Sidebar.Footer>
		<Sidebar.Menu class={compact ? 'gap-0' : 'gap-1'}>
			{#each footer as page (page.href)}
				{@render entry(page)}
			{/each}
		</Sidebar.Menu>
	</Sidebar.Footer>

	<Sidebar.Rail />
</Sidebar.Root>
