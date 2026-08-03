<script lang="ts">
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Info from '@lucide/svelte/icons/info';
	import Filter from '@lucide/svelte/icons/list-filter';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Users from '@lucide/svelte/icons/users';
	import { api, type SessionFacets, type SessionPage } from '$lib/api';
	import ActivityBadge from '$lib/components/activity-badge.svelte';
	import DateRangeMenu from '$lib/components/date-range-menu.svelte';
	import ResetView from '$lib/components/reset-view.svelte';
	import FilterMenu from '$lib/components/filter-menu.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Table from '$lib/components/ui/table';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { compact, displayPath, exact, formatWhen } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import type { MessageKey } from '$lib/i18n/en';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { nav } from '$lib/nav.svelte';
	import { FAMILIES, costOf, rates } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';

	type Column = {
		id: string;
		label: MessageKey;
		/** Sortable columns name the key the host understands. */
		sort?: string;
		numeric?: boolean;
		/** One hide rule, applied to the header and the cell alike. */
		class?: string;
		info: MessageKey;
	};

	const COLUMNS: Column[] = [
		{ id: 'topic', label: 'sessions.column.topic', sort: 'topic', info: 'info.sessions' },
		{
			id: 'project',
			label: 'sessions.column.project',
			sort: 'project',
			info: 'info.status',
			class: 'hidden @xl:table-cell'
		},
		{
			id: 'activity',
			label: 'sessions.column.activity',
			sort: 'activity',
			info: 'info.activity',
			class: 'w-36 hidden @md:table-cell'
		},
		{ id: 'last', label: 'sessions.column.last', sort: 'last', info: 'info.lastActive' },
		{ id: 'cost', label: 'sessions.column.cost', sort: 'cost', numeric: true, info: 'info.cost' },
		{
			id: 'duration',
			label: 'sessions.column.duration',
			sort: 'duration',
			numeric: true,
			info: 'info.duration',
			class: 'hidden @2xl:table-cell'
		},
		{
			id: 'turns',
			label: 'sessions.column.turns',
			sort: 'turns',
			numeric: true,
			info: 'info.turns',
			class: 'hidden @lg:table-cell'
		},
		{
			id: 'input',
			label: 'sessions.column.input',
			sort: 'input',
			numeric: true,
			info: 'info.input',
			class: 'hidden @3xl:table-cell'
		},
		{
			id: 'output',
			label: 'sessions.column.output',
			sort: 'output',
			numeric: true,
			info: 'info.output',
			class: 'hidden @3xl:table-cell'
		},
		{
			id: 'cacheRead',
			label: 'sessions.column.cache',
			sort: 'cacheRead',
			numeric: true,
			info: 'info.cacheRead',
			class: 'hidden @4xl:table-cell'
		},
		{
			id: 'model',
			label: 'sessions.column.model',
			sort: 'model',
			info: 'info.model',
			class: 'hidden @2xl:table-cell'
		}
	];

	const CLASS: Record<string, string> = Object.fromEntries(
		COLUMNS.map((column) => [column.id, column.class ?? ''])
	);

	const PAGE_SIZES = [25, 50, 100, 250];

	// The filters are the host's work, not the page's: it searches, filters and
	// sorts the whole history and hands back one page of the result. Changing
	// how many rows that page holds changes nothing about what was searched.
	let pageSize = $state(25);
	let page = $state(0);
	let sort = $state('last');
	let descending = $state(true);
	let searchInput = $state('');
	let search = $state('');
	// The activity page arrives here with its filter already chosen. Taken once,
	// as the starting value: reading it again would undo the reader's next click
	// on the very filter they came in through.
	const arriving = nav.take();

	let activities = $state<string[]>(arriving ? [arriving] : []);
	let models = $state<string[]>([]);
	let tags = $state<string[]>([]);
	let projects = $state<string[]>([]);
	let branches = $state<string[]>([]);
	let from = $state<string | null>(null);
	let to = $state<string | null>(null);

	let result = $state<SessionPage | null>(null);
	let facets = $state<SessionFacets | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// The host does the filtering, so every keystroke would be a query. Waiting
	// for a pause keeps a long history responsive while typing.
	let debounce: ReturnType<typeof setTimeout>;
	function onSearchInput() {
		clearTimeout(debounce);
		debounce = setTimeout(() => {
			search = searchInput;
			page = 0;
		}, 250);
	}

	function toggleSort(column: Column) {
		if (!column.sort) return;
		if (sort === column.sort) {
			descending = !descending;
		} else {
			sort = column.sort;
			descending = true;
		}
		page = 0;
	}

	/** Every filter resets paging: page 4 of a narrower list is rarely page 4. */
	function toggle(list: string[], value: string): string[] {
		page = 0;
		return list.includes(value) ? list.filter((entry) => entry !== value) : [...list, value];
	}

	$effect(() => {
		const query = {
			page,
			pageSize,
			sort,
			descending,
			search: search || null,
			activities,
			models,
			tags,
			projects,
			branches,
			from,
			to,
			// Sorting by cost happens in the host, over the whole history rather
			// than over the page — so it needs the table the window prices with.
			rates: FAMILIES.map((family) => ({ family: family.id, ...rates.for(family.id) }))
		};
		void scan.dataVersion;

		if (!isHosted) {
			loading = false;
			return;
		}

		let cancelled = false;
		error = null;
		api
			.listSessions(query)
			.then((value) => {
				if (!cancelled) result = value;
			})
			.catch((cause) => {
				if (!cancelled) error = errorMessage(cause);
			})
			.finally(() => {
				if (!cancelled) loading = false;
			});

		return () => {
			cancelled = true;
		};
	});

	$effect(() => {
		void scan.dataVersion;
		if (isHosted) void api.getSessionFacets().then((value) => (facets = value));
	});

	function columnFilter(id: string) {
		if (!facets) return null;
		switch (id) {
			case 'project':
				return {
					options: facets.projects,
					chosen: projects,
					display: (value: string) => value,
					toggle: (value: string) => (projects = toggle(projects, value)),
					clear: () => {
						projects = [];
						page = 0;
					}
				};
			case 'model':
				return {
					options: facets.models,
					chosen: models,
					display: shortModel,
					toggle: (value: string) => (models = toggle(models, value)),
					clear: () => {
						models = [];
						page = 0;
					}
				};
			case 'activity':
				return {
					options: facets.activities,
					chosen: activities,
					display: activityLabel,
					toggle: (value: string) => (activities = toggle(activities, value)),
					clear: () => {
						activities = [];
						page = 0;
					}
				};
			default:
				return null;
		}
	}

	function resetView() {
		searchInput = '';
		search = '';
		activities = [];
		models = [];
		tags = [];
		projects = [];
		branches = [];
		from = null;
		to = null;
		page = 0;
	}

	const hasFilters = $derived(
		Boolean(search) ||
			activities.length + models.length + tags.length + projects.length + branches.length > 0 ||
			Boolean(from || to)
	);

	const totalPages = $derived(result ? Math.max(1, Math.ceil(result.total / result.pageSize)) : 1);

	function formatDuration(minutes: number): string {
		if (minutes < 1) return '<1m';
		if (minutes < 60) return `${minutes}m`;
		return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
	}

	/** The table knows the topic; the header would otherwise show a bare UUID. */
	function openSession(sessionId: string, label: string) {
		nav.detailLabel = label;
		void goto(resolve('/sessions/[id]', { id: sessionId }));
	}

	function activityLabel(value: string): string {
		return t(`activity.${value}` as MessageKey);
	}

	function shortModel(model: string | null): string {
		return model ? model.replace(/^claude-/, '') : '—';
	}
</script>

<div class="@container flex h-full flex-col gap-3 p-4">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">
			{t('common.noHost')}
		</p>
	{:else}
		<div class="flex shrink-0 flex-wrap items-center gap-2">
			<Input
				placeholder={t('sessions.search')}
				class="h-8 max-w-xs"
				bind:value={searchInput}
				oninput={onSearchInput}
			/>
			<ResetView show={hasFilters} onreset={resetView} />

			{#if facets}
				<FilterMenu
					label={t('filter.project')}
					options={facets.projects}
					chosen={projects}
					onToggle={(value: string) => (projects = toggle(projects, value))}
					onClear={() => {
						projects = [];
						page = 0;
					}}
				/>
				<FilterMenu
					label={t('filter.branch')}
					options={facets.branches}
					chosen={branches}
					onToggle={(value: string) => (branches = toggle(branches, value))}
					onClear={() => {
						branches = [];
						page = 0;
					}}
				/>
				<FilterMenu
					label={t('filter.model')}
					options={facets.models}
					chosen={models}
					display={shortModel}
					onToggle={(value: string) => (models = toggle(models, value))}
					onClear={() => {
						models = [];
						page = 0;
					}}
				/>
				<FilterMenu
					label={t('filter.activity')}
					options={facets.activities}
					chosen={activities}
					display={activityLabel}
					onToggle={(value: string) => (activities = toggle(activities, value))}
					onClear={() => {
						activities = [];
						page = 0;
					}}
				/>
				{#if facets.tags.length > 0}
					<FilterMenu
						label={t('filter.tag')}
						options={facets.tags}
						chosen={tags}
						onToggle={(value: string) => (tags = toggle(tags, value))}
						onClear={() => {
							tags = [];
							page = 0;
						}}
					/>
				{/if}
			{/if}

			<DateRangeMenu
				{from}
				{to}
				onChange={(nextFrom: string | null, nextTo: string | null) => {
					from = nextFrom;
					to = nextTo;
					page = 0;
				}}
			/>

			{#if result}
				<span class="ml-auto text-xs text-muted-foreground tabular-nums">
					{t('sessions.count', { count: exact(result.total) })}
				</span>
			{/if}
		</div>

		{#if error}
			<Card.Root>
				<Card.Header>
					<Card.Title>{t('sessions.loadFailed')}</Card.Title>
					<Card.Description class="font-mono text-xs">{error}</Card.Description>
				</Card.Header>
			</Card.Root>
		{:else if loading && !result}
			<div class="flex flex-col gap-2">
				{#each [...Array(8).keys()] as index (index)}
					<Skeleton class="h-10 w-full" />
				{/each}
			</div>
		{:else if result && result.rows.length === 0}
			<Card.Root>
				<Card.Header>
					<Card.Title>{t('sessions.emptyTitle')}</Card.Title>
					<Card.Description>
						{hasFilters ? t('sessions.emptyFiltered') : t('sessions.emptyScan')}
					</Card.Description>
				</Card.Header>
			</Card.Root>
		{:else if result}
			<div
				class="min-h-0 flex-1 overflow-auto rounded-md border [&_td]:py-1 [&_td]:text-[13px] [&_th]:h-8 [&>[data-slot=table-container]]:overflow-visible"
			>
				<Table.Root>
					<Table.Header class="sticky top-0 z-10 bg-background">
						<Table.Row>
							{#each COLUMNS as column (column.id)}
								<Table.Head class={[column.class, column.numeric && 'text-right']}>
									<div class={['flex items-center gap-1', column.numeric && 'justify-end']}>
										{#if column.sort}
											<button
												type="button"
												class="inline-flex items-center gap-1 transition-colors hover:text-foreground"
												title={t('common.multiSortHint')}
												onclick={() => toggleSort(column)}
											>
												{t(column.label)}
												{#if sort === column.sort}
													{#if descending}
														<ArrowDown class="size-3" />
													{:else}
														<ArrowUp class="size-3" />
													{/if}
												{/if}
											</button>
										{:else}
											{t(column.label)}
										{/if}

										<Tooltip.Root>
											<Tooltip.Trigger>
												{#snippet child({ props })}
													<button
														{...props}
														type="button"
														aria-label={`${t(column.label)} — ${t('common.whatIsThis')}`}
														class="rounded p-0.5 text-muted-foreground/50 transition-colors hover:text-foreground"
													>
														<Info class="size-3" />
													</button>
												{/snippet}
											</Tooltip.Trigger>
											<Tooltip.Content class="max-w-72 text-xs font-normal">
												{t(column.info)}
											</Tooltip.Content>
										</Tooltip.Root>

										{#if facets && columnFilter(column.id)}
											{@const filter = columnFilter(column.id)!}
											<DropdownMenu.Root>
												<DropdownMenu.Trigger>
													{#snippet child({ props })}
														<button
															{...props}
															type="button"
															aria-label={`${t(column.label)} — ${t('common.filter')}`}
															class={[
																'rounded p-0.5 transition-colors hover:text-foreground',
																filter.chosen.length > 0
																	? 'text-primary'
																	: 'text-muted-foreground/60'
															]}
														>
															<Filter class="size-3" />
														</button>
													{/snippet}
												</DropdownMenu.Trigger>

												<DropdownMenu.Content class="max-h-80 w-56 overflow-y-auto">
													<DropdownMenu.Label class="flex items-center justify-between gap-2">
														{t('common.filter')}
														{#if filter.chosen.length > 0}
															<button
																type="button"
																class="text-xs font-normal text-muted-foreground hover:text-foreground"
																onclick={filter.clear}
															>
																{t('common.clearFilter')}
															</button>
														{/if}
													</DropdownMenu.Label>
													<DropdownMenu.Separator />

													{#each filter.options as option (option)}
														<DropdownMenu.CheckboxItem
															checked={filter.chosen.includes(option)}
															onCheckedChange={() => filter.toggle(option)}
															closeOnSelect={false}
														>
															<span class="truncate">{filter.display(option)}</span>
														</DropdownMenu.CheckboxItem>
													{/each}
												</DropdownMenu.Content>
											</DropdownMenu.Root>
										{/if}
									</div>
								</Table.Head>
							{/each}
						</Table.Row>
					</Table.Header>

					<Table.Body>
						{#each result.rows as row (row.sessionId)}
							<Table.Row
								class="cursor-pointer"
								onclick={() => openSession(row.sessionId, row.topic ?? row.sessionId.slice(0, 8))}
							>
								<Table.Cell class="max-w-[16rem] @2xl:max-w-[22rem]">
									<div class="flex items-center gap-2">
										<span class="truncate font-medium">
											{row.topic ?? row.sessionId.slice(0, 8)}
										</span>
										{#if row.hasSubagents}
											<Tooltip.Root>
												<Tooltip.Trigger>
													{#snippet child({ props })}
														<Users {...props} class="size-3.5 shrink-0 text-muted-foreground" />
													{/snippet}
												</Tooltip.Trigger>
												<Tooltip.Content>{t('sessions.subagents')}</Tooltip.Content>
											</Tooltip.Root>
										{/if}
									</div>
									{#if row.tags.length > 0}
										<div class="mt-1 flex flex-wrap gap-1">
											{#each row.tags as tag (tag)}
												<Badge variant="outline" class="h-4 px-1 text-[10px] font-normal">
													{tag}
												</Badge>
											{/each}
										</div>
									{/if}
								</Table.Cell>

								<Table.Cell class={[CLASS.project, 'max-w-[12rem] text-muted-foreground']}>
									<span class="block truncate"
										>{row.projectName ? displayPath(row.projectName) : '—'}</span
									>
									{#if row.gitBranch}
										<span class="block truncate font-mono text-xs opacity-70">{row.gitBranch}</span>
									{/if}
								</Table.Cell>

								<Table.Cell class={CLASS.activity}>
									<ActivityBadge activity={row.activity} profile={row.profile} />
								</Table.Cell>

								<Table.Cell class="whitespace-nowrap text-muted-foreground">
									{formatWhen(row.lastTs)}
								</Table.Cell>
								<Table.Cell class="text-right whitespace-nowrap tabular-nums">
									{row.model ? region.format(costOf(row.model, row)) : t('common.none')}
								</Table.Cell>
								<Table.Cell class={[CLASS.duration, 'text-right whitespace-nowrap tabular-nums']}>
									{formatDuration(row.durationMinutes)}
								</Table.Cell>
								<Table.Cell class={[CLASS.turns, 'text-right tabular-nums']}>
									{row.turnCount}
								</Table.Cell>
								<Table.Cell
									class={[CLASS.input, 'text-right tabular-nums']}
									title={exact(row.inputTokens)}
								>
									{compact(row.inputTokens)}
								</Table.Cell>
								<Table.Cell
									class={[CLASS.output, 'text-right tabular-nums']}
									title={exact(row.outputTokens)}
								>
									{compact(row.outputTokens)}
								</Table.Cell>
								<Table.Cell
									class={[CLASS.cacheRead, 'text-right tabular-nums']}
									title={exact(row.cacheReadTokens)}
								>
									{compact(row.cacheReadTokens)}
								</Table.Cell>
								<Table.Cell class={[CLASS.model, 'whitespace-nowrap text-muted-foreground']}>
									{shortModel(row.model)}
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			</div>

			<div class="flex shrink-0 flex-wrap items-center justify-between gap-2">
				<div class="flex items-center gap-2">
					<span class="text-xs text-muted-foreground tabular-nums">
						{t('sessions.page', { page: result.page + 1, total: totalPages })}
					</span>

					<DropdownMenu.Root>
						<DropdownMenu.Trigger>
							{#snippet child({ props })}
								<Button {...props} variant="outline" size="sm" class="h-7 gap-1 font-normal">
									{t('sessions.perPage', { count: pageSize })}
									<ChevronDown class="size-3.5 opacity-60" />
								</Button>
							{/snippet}
						</DropdownMenu.Trigger>
						<DropdownMenu.Content align="start" class="w-40">
							{#each PAGE_SIZES as size (size)}
								<DropdownMenu.Item
									onSelect={() => {
										pageSize = size;
										page = 0;
									}}
								>
									<span class="flex-1">{t('sessions.perPage', { count: size })}</span>
									{#if pageSize === size}
										<Check class="size-4" />
									{/if}
								</DropdownMenu.Item>
							{/each}
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				</div>
				<div class="flex gap-1">
					<Button
						variant="outline"
						size="icon"
						class="size-7"
						disabled={result.page === 0}
						onclick={() => (page = Math.max(0, page - 1))}
						aria-label={t('sessions.previousPage')}
					>
						<ChevronLeft />
					</Button>
					<Button
						variant="outline"
						size="icon"
						class="size-7"
						disabled={result.page + 1 >= totalPages}
						onclick={() => (page = page + 1)}
						aria-label={t('sessions.nextPage')}
					>
						<ChevronRight />
					</Button>
				</div>
			</div>
		{/if}
	{/if}
</div>
