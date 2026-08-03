<script lang="ts">
	import { resolve } from '$app/paths';
	import RailToggle from '$lib/components/rail-toggle.svelte';
	import { t } from '$lib/i18n/index.svelte';
	import { cn } from '$lib/utils';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import type { PageDefinition } from '$lib/pages';

	type Props = {
		pages: readonly PageDefinition[];
		settingsPage: PageDefinition;
		active: string;
	};

	let { pages, settingsPage, active }: Props = $props();

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

	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each pages as page (page.href)}
						<Sidebar.MenuItem>
							<Tooltip.Root>
								<Tooltip.Trigger>
									{#snippet child({ props })}
										<Sidebar.MenuButton {...props} isActive={active === page.href}>
											{#snippet child({ props: buttonProps })}
												<a
													{...buttonProps}
													href={resolve(page.href)}
													class={cn((buttonProps as { class?: string }).class, HOVER)}
												>
													<page.icon />
													<span>{t(page.label)}</span>
												</a>
											{/snippet}
										</Sidebar.MenuButton>
									{/snippet}
								</Tooltip.Trigger>
								<Tooltip.Content side="right">{t(page.label)}</Tooltip.Content>
							</Tooltip.Root>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	</Sidebar.Content>

	<Sidebar.Footer>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<Sidebar.MenuButton {...props} isActive={active === settingsPage.href}>
								{#snippet child({ props: buttonProps })}
									<a
										{...buttonProps}
										href={resolve(settingsPage.href)}
										class={cn((buttonProps as { class?: string }).class, HOVER)}
									>
										<settingsPage.icon />
										<span>{t(settingsPage.label)}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content side="right">{t(settingsPage.label)}</Tooltip.Content>
				</Tooltip.Root>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Footer>

	<Sidebar.Rail />
</Sidebar.Root>
