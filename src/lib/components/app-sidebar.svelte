<script lang="ts">
	import { resolve } from '$app/paths';
	import { t } from '$lib/i18n/index.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import type { PageDefinition } from '$lib/pages';

	type Props = {
		pages: readonly PageDefinition[];
		settingsPage: PageDefinition;
		active: string;
	};

	let { pages, settingsPage, active }: Props = $props();
</script>

<Sidebar.Root collapsible="icon">
	<Sidebar.Header>
		<div class="flex h-8 items-center gap-1 px-1">
			<!-- The collapse control sits where the eye lands first, and takes the
			     place the mark used to occupy: a logo there was a target that did
			     nothing. -->
			<Sidebar.Trigger class="shrink-0" />
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
												<a {...buttonProps} href={resolve(page.href)}>
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
									<a {...buttonProps} href={resolve(settingsPage.href)}>
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
