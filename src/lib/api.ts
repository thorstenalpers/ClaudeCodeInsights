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

export const api = {
  getOverview: () => invoke<Overview>('get_overview'),
  getScanState: () => invoke<ScanState>('get_scan_state'),
  startScan: () => invoke<boolean>('start_scan'),
};
