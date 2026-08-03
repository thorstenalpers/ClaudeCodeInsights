<script lang="ts">
	/**
	 * The projects, what is meant to happen in them, and what is there to do it.
	 *
	 * The list of tasks is the app's own — features, stories, whatever is written
	 * down — and it is what a session will eventually be started for. The rules
	 * beneath it are stored but not yet obeyed: nothing runs sessions from here
	 * until the session runner exists, and saying so is better than a switch that
	 * quietly does nothing.
	 */
	import { api, type Definition, type ProjectRow, type Task } from '$lib/api';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { compact, displayPath, exact, formatWhen } from '$lib/format';
	import { t } from '$lib/i18n/index.svelte';
	import { errorMessage, isHosted } from '$lib/ipc.svelte';
	import { costOfSplit } from '$lib/pricing.svelte';
	import { region } from '$lib/region.svelte';
	import { scan } from '$lib/scan.svelte';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash from '@lucide/svelte/icons/trash-2';

	let projects = $state<ProjectRow[] | null>(null);
	let tasks = $state<Task[]>([]);
	let skills = $state<Definition[]>([]);
	let agents = $state<Definition[]>([]);
	let chosen = $state<string>('');
	let title = $state('');
	let error = $state<string | null>(null);

	$effect(() => {
		void scan.dataVersion;
		if (!isHosted) return;

		let cancelled = false;
		api
			.listProjects()
			.then((report) => {
				if (cancelled) return;
				projects = report.projects;
				if (chosen === '' && report.projects.length > 0) chosen = report.projects[0].path;
			})
			.catch((cause) => {
				if (!cancelled) error = errorMessage(cause);
			});

		return () => {
			cancelled = true;
		};
	});

	$effect(() => {
		const project = chosen;
		if (!isHosted || project === '') return;

		let cancelled = false;
		void Promise.all([api.listTasks(project), api.listDefinitions(project)]).then(
			([list, [foundSkills, foundAgents]]) => {
				if (cancelled) return;
				tasks = list;
				skills = foundSkills;
				agents = foundAgents;
			}
		);

		return () => {
			cancelled = true;
		};
	});

	/** The states a task moves through, in the order the buttons offer them. */
	const STATES = ['open', 'running', 'waiting', 'deferred', 'done'] as const;

	const TONE: Record<string, string> = {
		open: 'outline',
		running: 'default',
		waiting: 'secondary',
		deferred: 'secondary',
		done: 'secondary'
	};

	async function reload(): Promise<void> {
		if (chosen === '') return;
		tasks = await api.listTasks(chosen).catch(() => tasks);
	}

	async function add(): Promise<void> {
		if (chosen === '' || title.trim() === '') return;
		await api.addTask(chosen, title).catch((cause) => (error = errorMessage(cause)));
		title = '';
		await reload();
	}

	async function move(task: Task, state: string): Promise<void> {
		await api.updateTask({ ...task, state });
		await reload();
	}

	async function remove(task: Task): Promise<void> {
		await api.removeTask(task.id);
		await reload();
	}

	const current = $derived(projects?.find((project) => project.path === chosen) ?? null);

	/**
	 * The rules a supervisor would follow once there is one to follow them.
	 *
	 * Kept next to the tasks because that is where they will apply; each is off
	 * until the session runner can honour it.
	 */
	const RULES = [
		'orchestration.rule.defer',
		'orchestration.rule.ask',
		'orchestration.rule.stopOnBudget',
		'orchestration.rule.oneAtATime'
	] as const;

	const RULES_KEY = 'claudeadmin.orchestration.rules';
	let rules = $state<string[]>(readRules());

	function readRules(): string[] {
		if (typeof localStorage === 'undefined') return [];
		try {
			const stored: unknown = JSON.parse(localStorage.getItem(RULES_KEY) ?? '[]');
			return Array.isArray(stored) ? stored.filter((rule) => typeof rule === 'string') : [];
		} catch {
			return [];
		}
	}

	function toggleRule(rule: string): void {
		rules = rules.includes(rule) ? rules.filter((entry) => entry !== rule) : [...rules, rule];
		localStorage.setItem(RULES_KEY, JSON.stringify(rules));
	}
</script>

<div class="@container flex h-full min-h-0 flex-col gap-3 p-4">
	{#if !isHosted}
		<p class="text-sm text-muted-foreground">{t('common.noHost')}</p>
	{:else if error}
		<Card.Root>
			<Card.Header>
				<Card.Title>{t('overview.dbFailed')}</Card.Title>
				<Card.Description class="font-mono text-xs">{error}</Card.Description>
			</Card.Header>
		</Card.Root>
	{:else if !projects}
		<Skeleton class="h-64 w-full" />
	{:else}
		<div class="flex shrink-0 gap-1 overflow-x-auto border-b pb-1">
			{#each projects as project (project.path)}
				<Button
					variant={chosen === project.path ? 'secondary' : 'ghost'}
					size="sm"
					class="shrink-0 font-normal"
					onclick={() => (chosen = project.path)}
				>
					<span class="max-w-48 truncate">{displayPath(project.path)}</span>
					<span class="text-xs text-muted-foreground tabular-nums">{project.sessions}</span>
				</Button>
			{/each}
		</div>

		<div class="grid min-h-0 flex-1 gap-3 overflow-auto @3xl:grid-cols-2">
			<Card.Root data-size="sm">
				<Card.Header class="gap-1">
					<Card.Title class="text-base">{t('orchestration.tasks')}</Card.Title>
					<Card.Description>{t('orchestration.tasks.description')}</Card.Description>
				</Card.Header>
				<Card.Content class="flex flex-col gap-2">
					<form
						class="flex flex-wrap items-center gap-2"
						onsubmit={(event) => {
							event.preventDefault();
							void add();
						}}
					>
						<Input
							class="h-8 flex-1"
							bind:value={title}
							placeholder={t('orchestration.tasks.placeholder')}
						/>
						<Button type="submit" variant="outline" size="sm" class="h-8 gap-1 font-normal">
							<Plus class="size-3.5" />
							{t('orchestration.tasks.add')}
						</Button>
					</form>

					{#each tasks as task (task.id)}
						<div class="flex flex-col gap-1 border-b pb-2 last:border-0 last:pb-0">
							<div class="flex flex-wrap items-center gap-2">
								<span class="min-w-0 flex-1 truncate text-sm">{task.title}</span>
								<Badge
									variant={TONE[task.state] === 'default' ? 'default' : 'outline'}
									class="font-normal"
								>
									{t(`orchestration.state.${task.state}` as 'orchestration.state.open')}
								</Badge>
								<Button
									variant="ghost"
									size="sm"
									class="h-7 px-2"
									aria-label={t('orchestration.tasks.remove')}
									onclick={() => void remove(task)}
								>
									<Trash class="size-3.5" />
								</Button>
							</div>
							<div class="flex flex-wrap gap-1">
								{#each STATES as state (state)}
									<Button
										variant={task.state === state ? 'secondary' : 'ghost'}
										size="sm"
										class="h-6 px-2 text-xs font-normal"
										onclick={() => void move(task, state)}
									>
										{t(`orchestration.state.${state}` as 'orchestration.state.open')}
									</Button>
								{/each}
								{#if task.sessionId}
									<span class="ml-auto font-mono text-xs text-muted-foreground">
										{task.sessionId.slice(0, 8)}
									</span>
								{/if}
							</div>
						</div>
					{/each}

					{#if tasks.length === 0}
						<p class="text-xs text-muted-foreground">{t('orchestration.tasks.empty')}</p>
					{/if}
				</Card.Content>
			</Card.Root>

			<div class="flex min-h-0 flex-col gap-3">
				<Card.Root data-size="sm">
					<Card.Header class="gap-1">
						<Card.Title class="text-base">{t('orchestration.project')}</Card.Title>
					</Card.Header>
					<Card.Content class="flex flex-wrap items-center gap-2">
						{#if current}
							{@const cost = costOfSplit(current.byModel)}
							<Badge variant="outline" class="font-normal">
								{t('cost.column.sessions')}: {exact(current.sessions)}
							</Badge>
							<Badge variant="outline" class="font-normal">
								{t('cost.column.turns')}: {exact(current.turns)}
							</Badge>
							<Badge variant="outline" class="font-normal">
								{t('agents.column.tokens')}: {compact(
									current.inputTokens + current.outputTokens + current.cacheReadTokens
								)}
							</Badge>
							<Badge variant="outline" class="font-normal">
								{t('cost.column.cost')}: {cost === null ? t('common.none') : region.format(cost)}
							</Badge>
							<span class="ml-auto text-xs text-muted-foreground">
								{formatWhen(current.lastTs)}
							</span>
						{/if}
					</Card.Content>
				</Card.Root>

				<Card.Root data-size="sm">
					<Card.Header class="gap-1">
						<Card.Title class="text-base">{t('orchestration.rules')}</Card.Title>
						<Card.Description>{t('orchestration.rules.description')}</Card.Description>
					</Card.Header>
					<Card.Content class="flex flex-col gap-2">
						{#each RULES as rule (rule)}
							<button
								type="button"
								aria-pressed={rules.includes(rule)}
								class="flex items-start gap-2 rounded-md border p-2 text-left transition-colors hover:bg-primary/10"
								class:border-primary={rules.includes(rule)}
								onclick={() => toggleRule(rule)}
							>
								<span
									class="mt-0.5 size-4 shrink-0 rounded-sm border-2"
									class:bg-primary={rules.includes(rule)}
									class:border-primary={rules.includes(rule)}
								></span>
								<span class="text-sm">{t(rule)}</span>
							</button>
						{/each}
					</Card.Content>
				</Card.Root>

				<Card.Root data-size="sm">
					<Card.Header class="gap-1">
						<Card.Title class="text-base">{t('orchestration.available')}</Card.Title>
						<Card.Description>{t('orchestration.available.description')}</Card.Description>
					</Card.Header>
					<Card.Content class="flex flex-col gap-2">
						{#each [{ label: 'orchestration.skills', list: skills }, { label: 'orchestration.agents', list: agents }] as group (group.label)}
							<div class="flex flex-col gap-1">
								<span class="text-xs text-muted-foreground">
									{t(group.label as 'orchestration.skills')} · {group.list.length}
								</span>
								{#each group.list.slice(0, 8) as entry (entry.path)}
									<div class="flex items-center gap-2">
										<span class="shrink-0 text-sm">{entry.name}</span>
										<span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
											{entry.description}
										</span>
										<Badge variant="outline" class="shrink-0 font-normal">
											{entry.scope === 'user' ? t('orchestration.scope.user') : t('nav.projects')}
										</Badge>
									</div>
								{/each}
							</div>
						{/each}
					</Card.Content>
				</Card.Root>
			</div>
		</div>
	{/if}
</div>
