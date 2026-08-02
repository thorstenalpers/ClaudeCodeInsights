<script lang="ts">
	import { resolve } from '$app/paths';
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
			<!-- The app mark: the same three ascending bars as the icon and the splash. -->
			<div class="flex h-5 shrink-0 items-end gap-[3px]" aria-hidden="true">
				<span class="h-2 w-[3px] rounded-[1px] bg-foreground"></span>
				<span class="h-3.5 w-[3px] rounded-[1px] bg-foreground"></span>
				<span class="h-5 w-[3px] rounded-[1px] bg-foreground"></span>
			</div>
			<span
				class="truncate text-sm font-semibold tracking-tight group-data-[collapsible=icon]:hidden"
			>
				ClaudeAdmin
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
