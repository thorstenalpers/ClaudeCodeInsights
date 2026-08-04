import { invoke } from './ipc.svelte';

export type Overview = {
	sessions: number;
	turns: number;
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheWriteTokens: number;
	activeDays: number;
	firstTs: string | null;
	lastTs: string | null;
};

export type ScanState = {
	running: boolean;
	roots: string[];
};

export type ScanProgress = {
	filesDone: number;
	filesTotal: number;
	currentFile: string | null;
};

export type ScanStats = {
	filesTotal: number;
	filesRead: number;
	filesSkipped: number;
	turnsInserted: number;
	sessionsSeen: number;
	malformedLines: number;
	filesFailed: number;
	firstError: string | null;
};

export type SessionRow = {
	sessionId: string;
	topic: string | null;
	projectName: string | null;
	gitBranch: string | null;
	firstTs: string | null;
	lastTs: string | null;
	durationMinutes: number;
	model: string | null;
	turnCount: number;
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheWriteTokens: number;
	hasSubagents: boolean;
	activity: string;
	/** Share per tool category; can sum above 1 because a tool may be in several. */
	profile: Record<string, number>;
	tags: string[];
};

export type SessionPage = {
	rows: SessionRow[];
	total: number;
	page: number;
	pageSize: number;
};

export type SessionQuery = {
	page?: number;
	pageSize?: number;
	sort?: string;
	descending?: boolean;
	search?: string | null;
	activities?: string[];
	models?: string[];
	tags?: string[];
	projects?: string[];
	branches?: string[];
	/** Inclusive local dates, `YYYY-MM-DD`. */
	from?: string | null;
	to?: string | null;
	/** The price table, sent only so the host can order by cost. */
	rates?: {
		family: string;
		input: number;
		output: number;
		cacheRead: number;
		cacheWrite: number;
	}[];
};

export type SessionFacets = {
	models: string[];
	activities: string[];
	tags: string[];
	projects: string[];
	branches: string[];
};

export type ToolCall = {
	name: string;
	input: string;
	inputTruncated: boolean;
	result: string | null;
	resultTruncated: boolean;
	isError: boolean;
};

export type TranscriptTurn = {
	index: number;
	role: 'user' | 'assistant';
	timestamp: string | null;
	text: string | null;
	thinking: string | null;
	toolCalls: ToolCall[];
};

export type TranscriptPage = {
	turns: TranscriptTurn[];
	total: number;
	offset: number;
	path: string | null;
};

/** Tokens a row spent on one model, so a mixed row can still be priced. */
export type ModelTokens = {
	model: string;
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheWriteTokens: number;
};

export type ProjectRow = {
	path: string;
	registered: boolean;
	dirExists: boolean;
	duplicateGroup: string | null;
	transcriptDir: string | null;
	transcriptFiles: number;
	transcriptBytes: number;
	sessions: number;
	turns: number;
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheWriteTokens: number;
	lastTs: string | null;
	byModel: ModelTokens[];
};

export type ProjectsReport = {
	configPath: string;
	configExists: boolean;
	projects: ProjectRow[];
};

export type TranscriptFile = {
	path: string;
	name: string;
	sizeBytes: number;
};

export type DeleteOutcome = {
	filesDeleted: number;
	bytesFreed: number;
	failed: string[];
};

export type WriteOutcome = {
	backupPath: string;
};

export type ToolRow = {
	name: string;
	calls: number;
	sessions: number;
	/** Each turn's tokens shared over the tool calls it made. */
	byModel: ModelTokens[];
};

export type ModelRow = {
	model: string;
	turns: number;
	sessions: number;
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheWriteTokens: number;
	firstTs: string | null;
	lastTs: string | null;
};

/** What came back, and what actually served it. */
export type Answer = {
	text: string;
	model: string | null;
	effort: string | null;
	costUsd: number | null;
};

/** A name and how many runs fell under it. */
export type Tally = { name: string; runs: number };

export type AgentRow = {
	agentType: string;
	runs: number;
	totalTokens: number;
	totalDurationMs: number;
	toolUseCount: number;
	lastTs: string | null;
	/** The sessions that started runs of this type. */
	sessions: number;
	/** Where it ran and what those sessions were doing, busiest first. */
	projects: Tally[];
	activities: Tally[];
	/** Empty when no turn carries the agent's id, which is not the same as free. */
	byModel: ModelTokens[];
};

/** One derived activity, with everything the sessions behind it add up to. */
export type ActivityRow = {
	activity: string;
	sessions: number;
	turns: number;
	projects: number;
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheWriteTokens: number;
	firstTs: string | null;
	lastTs: string | null;
	byModel: ModelTokens[];
};

export type DayRow = { date: string; turns: number };

export type SeriesPoint = {
	date: string;
	key: string;
	model: string;
	turns: number;
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheWriteTokens: number;
};

export type Series = { points: SeriesPoint[]; keys: string[] };

export type SeriesQuery = {
	groupBy: 'model' | 'activity' | 'tool' | 'project' | 'branch' | 'none';
	models?: string[];
	activities?: string[];
	tools?: string[];
	projects?: string[];
	branches?: string[];
	from?: string | null;
	to?: string | null;
};

export type SeriesFacets = {
	models: string[];
	activities: string[];
	tools: string[];
	projects: string[];
	branches: string[];
};

export type Rhythm = {
	/** 7 x 24 turn counts, Monday first, in local time. */
	grid: number[][];
	busiestHour: number | null;
	busiestWeekday: number | null;
	days: DayRow[];
	activeDays: number;
	longestStreak: number;
	currentStreak: number;
};

export type LiveTurn = {
	sessionId: string;
	project: string | null;
	gitBranch: string | null;
	role: string;
	timestamp: string | null;
	text: string | null;
	tools: string[];
	model: string | null;
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheWriteTokens: number;
	/** The line carried a thinking block. */
	thinking: boolean;
	/** `code` when it reached for a tool, `chat` when it only spoke. */
	kind: string;
	/** From a subagent's own thread rather than the conversation. */
	agent: boolean;
};

export type VoicePack = {
	id: string;
	language: string;
	label: string;
	megabytes: number;
	installed: boolean;
	voices: number;
};

/** A Hugging Face model, with the verdict on whether this app can speak it. */
export type HubVoice = {
	repo: string;
	likes: number;
	downloads: number;
	languages: string[];
	kind: 'kokoro' | 'vits' | null;
	megabytes: number;
	/** Null when installable; otherwise what the repository is missing. */
	blocked: string | null;
};

/** How far a run may go before it asks. */
export type Rule = 'ask' | 'readsFree';

/** What the window asks for when it starts a run. */
export type RunRequest = {
	project: string;
	prompt: string;
	model?: string | null;
	cliPath?: string | null;
	rule: Rule;
	/** Ends the run when the spend passes this, in US dollars. */
	budgetUsd?: number | null;
	taskId?: number | null;
};

export type RunInfo = {
	id: string;
	project: string;
	taskId: number | null;
	rule: Rule;
};

/** One line of a run, already flattened by the host. */
export type RunLine = {
	run: string;
	/** `text`, `thinking`, `tool`, `result`, `system` — or `you` for what was typed. */
	kind: string;
	text: string;
	tool: string | null;
	costUsd: number | null;
};

/** A tool waiting for an answer. */
export type Ask = {
	id: string;
	run: string;
	tool: string;
	input: unknown;
};

/** One thing to be done in a project. */
export type Task = {
	id: number;
	projectPath: string;
	title: string;
	notes: string;
	/** `open`, `running`, `waiting`, `deferred` or `done`. */
	state: string;
	position: number;
	createdTs: string;
	sessionId: string | null;
};

/** A skill or an agent definition, as it lies on disk. */
export type Definition = {
	name: string;
	description: string;
	path: string;
	/** `user` for the home folder, otherwise the project it belongs to. */
	scope: string;
};

/** What the info page says about the running build. */
export type AppInfo = {
	version: string;
	dataDir: string;
	database: string;
	voices: string;
	logs: string;
};

/** One capture device, as Windows lists it. */
export type Microphone = { name: string; isDefault: boolean };

/** A speech model for the dictation that runs in this app. */
export type SpeechModel = {
	id: string;
	label: string;
	megabytes: number;
	installed: boolean;
};

export type CliStatus = { found: boolean; path: string | null; version: string | null };

export type ProviderInfo = {
	id: string;
	model: string;
	/** Set for providers that hand out a key for nothing. */
	freeKeyUrl: string | null;
};

export const api = {
	getOverview: () => invoke<Overview>('get_overview'),
	getAppInfo: () => invoke<AppInfo>('get_app_info'),
	openDataFolder: (which: string) => invoke<void>('open_data_folder', { which }),
	getTranscript: (sessionId: string, offset: number, limit: number) =>
		invoke<TranscriptPage>('get_transcript', { sessionId, offset, limit }),
	getScanState: () => invoke<ScanState>('get_scan_state'),
	startScan: () => invoke<boolean>('start_scan'),
	listSessions: (query: SessionQuery) => invoke<SessionPage>('list_sessions', { query }),
	getSessionFacets: () => invoke<SessionFacets>('get_session_facets'),
	listProjects: () => invoke<ProjectsReport>('list_projects'),
	openClaudeConfig: () => invoke<void>('open_claude_config'),
	previewProjectTranscripts: (path: string) =>
		invoke<TranscriptFile[]>('preview_project_transcripts', { path }),
	deleteProjectTranscripts: (path: string) =>
		invoke<DeleteOutcome>('delete_project_transcripts', { path }),
	getProjectSettings: (path: string) => invoke<string>('get_project_settings', { path }),
	updateProjectSettings: (path: string, settings: string) =>
		invoke<WriteOutcome>('update_project_settings', { path, settings }),
	removeProjectRegistration: (path: string) =>
		invoke<WriteOutcome>('remove_project_registration', { path }),
	listTools: () => invoke<ToolRow[]>('list_tools'),
	listActivities: () => invoke<ActivityRow[]>('list_activities'),
	startSession: (request: RunRequest) => invoke<RunInfo>('start_session', { request }),
	sendToSession: (id: string, prompt: string) => invoke<void>('send_to_session', { id, prompt }),
	interruptSession: (id: string) => invoke<void>('interrupt_session', { id }),
	stopSession: (id: string) => invoke<void>('stop_session', { id }),
	answerPermission: (id: string, allow: boolean) =>
		invoke<void>('answer_permission', { id, allow }),
	listRunningSessions: () => invoke<RunInfo[]>('list_sessions_running'),
	listTasks: (project: string | null) => invoke<Task[]>('list_tasks', { project }),
	addTask: (project: string, title: string) => invoke<Task>('add_task', { project, title }),
	updateTask: (task: Task) => invoke<void>('update_task', { task }),
	removeTask: (id: number) => invoke<void>('remove_task', { id }),
	listDefinitions: (project: string | null) =>
		invoke<[Definition[], Definition[]]>('list_definitions', { project }),
	listModels: () => invoke<ModelRow[]>('list_models'),
	listAgents: () => invoke<AgentRow[]>('list_agents'),
	getRhythm: () => invoke<Rhythm>('get_rhythm'),
	getSeries: (query: SeriesQuery) => invoke<Series>('get_series', { query }),
	getSeriesFacets: () => invoke<SeriesFacets>('get_series_facets'),
	getCliStatus: (path: string | null) => invoke<CliStatus>('get_cli_status', { path }),
	askClaude: (
		source: string,
		path: string | null,
		prompt: string,
		model: string | null,
		effort: string | null
	) => invoke<Answer>('ask_claude', { source, path, prompt, model, effort }),
	localOptions: () => invoke<[string[], string[]]>('local_options'),
	listProviders: () => invoke<ProviderInfo[]>('list_providers'),
	hasApiKey: (provider: string) => invoke<boolean>('has_api_key', { provider }),
	setApiKey: (provider: string, key: string) => invoke<void>('set_api_key', { provider, key }),
	openFreeKeyUrl: (provider: string) => invoke<void>('open_free_key_url', { provider }),
	speechAvailable: () => invoke<boolean>('speech_available'),
	listMicrophones: () => invoke<Microphone[]>('list_microphones'),
	openSoundSettings: () => invoke<void>('open_sound_settings'),
	startLive: (tail: number) => invoke<LiveTurn[]>('start_live', { tail }),
	stopLive: () => invoke<void>('stop_live'),
	listVoicePacks: () => invoke<VoicePack[]>('list_voice_packs'),
	voicePacksFolder: () => invoke<string>('voice_packs_folder'),
	installVoicePack: (id: string) => invoke<VoicePack>('install_voice_pack', { id }),
	searchVoiceHub: (query: string) => invoke<HubVoice[]>('search_voice_hub', { query }),
	installHubVoice: (repo: string) => invoke<VoicePack>('install_hub_voice', { repo }),
	cancelVoicePack: (id: string) => invoke<void>('cancel_voice_pack', { id }),
	removeVoicePack: (id: string) => invoke<void>('remove_voice_pack', { id }),
	speakText: (id: string, speaker: number, text: string) =>
		invoke<void>('speak_text', { id, speaker, text }),
	stopSpeaking: () => invoke<void>('stop_speaking'),
	openSpeechSettings: () => invoke<void>('open_speech_settings'),
	listSpeechLanguages: () => invoke<string[]>('list_speech_languages'),
	listSpeechModels: () => invoke<SpeechModel[]>('list_speech_models'),
	speechModelsFolder: () => invoke<string>('speech_models_folder'),
	installSpeechModel: (id: string) => invoke<SpeechModel>('install_speech_model', { id }),
	removeSpeechModel: (id: string) => invoke<void>('remove_speech_model', { id }),
	dictate: (device: string | null, model: string, locale: string) =>
		invoke<string>('dictate', { device, model, locale }),
	stopDictating: () => invoke<void>('stop_dictating'),
	recognizeSpeech: (locale: string) => invoke<string>('recognize_speech', { locale })
};
