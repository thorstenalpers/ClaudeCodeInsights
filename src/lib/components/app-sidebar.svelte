<script lang="ts">
	import { resolve } from '$app/paths';
	import AppMark from '$lib/components/app-mark.svelte';
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
		<div class="flex h-8 items-center gap-2 px-1">
			<AppMark class="size-5 shrink-0 text-foreground" />
			<span
				class="truncate text-sm font-semibold tracking-tight group-data-[collapsible=icon]:hidden"
			>
				Claude Admin
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
													<span>{page.label}</span>
												</a>
											{/snippet}
										</Sidebar.MenuButton>
									{/snippet}
								</Tooltip.Trigger>
								<Tooltip.Content side="right">{page.label}</Tooltip.Content>
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
										<span>{settingsPage.label}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content side="right">{settingsPage.label}</Tooltip.Content>
				</Tooltip.Root>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Footer>

	<Sidebar.Rail />
</Sidebar.Root>
