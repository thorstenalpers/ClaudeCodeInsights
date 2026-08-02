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
};

export type SessionFacets = {
	models: string[];
	activities: string[];
	tags: string[];
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

export const api = {
	getOverview: () => invoke<Overview>('get_overview'),
	getTranscript: (sessionId: string, offset: number, limit: number) =>
		invoke<TranscriptPage>('get_transcript', { sessionId, offset, limit }),
	getScanState: () => invoke<ScanState>('get_scan_state'),
	startScan: () => invoke<boolean>('start_scan'),
	listSessions: (query: SessionQuery) => invoke<SessionPage>('list_sessions', { query }),
	getSessionFacets: () => invoke<SessionFacets>('get_session_facets'),
	listProjects: () => invoke<ProjectsReport>('list_projects'),
	previewProjectTranscripts: (path: string) =>
		invoke<TranscriptFile[]>('preview_project_transcripts', { path }),
	deleteProjectTranscripts: (path: string) =>
		invoke<DeleteOutcome>('delete_project_transcripts', { path }),
	getProjectSettings: (path: string) => invoke<string>('get_project_settings', { path }),
	updateProjectSettings: (path: string, settings: string) =>
		invoke<WriteOutcome>('update_project_settings', { path, settings }),
	removeProjectRegistration: (path: string) =>
		invoke<WriteOutcome>('remove_project_registration', { path })
};
