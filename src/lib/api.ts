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

export const api = {
  getOverview: () => invoke<Overview>('get_overview'),
  getScanState: () => invoke<ScanState>('get_scan_state'),
  startScan: () => invoke<boolean>('start_scan'),
  listSessions: (query: SessionQuery) => invoke<SessionPage>('list_sessions', { query }),
  getSessionFacets: () => invoke<SessionFacets>('get_session_facets'),
};
