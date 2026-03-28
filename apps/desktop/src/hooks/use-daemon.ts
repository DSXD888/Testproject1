import { useSyncExternalStore } from 'react';

import { invoke } from '../lib/transport';
import {
  createSessionRegistry,
  type SessionFrame,
  type SessionListItem,
  type SessionRegistryApi,
} from '../stores/session-registry';
import {
  createSessionBuffer,
  type SessionBufferApi,
} from '../stores/session-buffer';

export type SessionListFilters = {
  phases?: SessionListItem['phase'][];
  executionModes?: SessionListItem['executionMode'][];
  activeOnly?: boolean;
  search?: string;
};

export type KeysetPage<T> = {
  items: T[];
  nextCursor: string | null;
  hasMore: boolean;
};

export type QueryEnvelope<T> = {
  data: T;
  snapshotSeq: number;
  snapshotAt: string;
};

export type MutationAck = {
  accepted: boolean;
  visibleAfterSeq: number;
  jobId?: string | null;
  sessionId?: string | null;
};

export type SessionHistoryRequest = {
  sessionId: string;
  fromSeq: number;
  toSeq: number;
  maxBytes?: number;
};

<<<<<<< HEAD
export type SessionHistoryFrame =
  | { kind: 'pty'; seq: number; text: string }
  | { kind: 'stdout'; seq: number; text: string }
  | { kind: 'stderr'; seq: number; text: string };

=======
>>>>>>> 73a88b6 (testproject1-dr6.22: add session viewer and replay flow)
export type SessionHistoryChunk = {
  sessionId: string;
  firstSeq: number;
  lastSeq: number;
<<<<<<< HEAD
  availability: 'complete' | 'partial' | 'missing';
  frames: SessionHistoryFrame[];
=======
  frames: SessionFrame[];
  availability: 'complete' | 'partial' | 'missing';
>>>>>>> 73a88b6 (testproject1-dr6.22: add session viewer and replay flow)
};

export type SessionMetaSnapshot = {
  session: SessionListItem | null;
  lastSeq: number;
  gapState: { fromSeq: number; toSeq: number } | null;
  archiveSeq: number | null;
};

export type SessionPageItem = SessionListItem & {
  jobId?: string | null;
};

const sessionRegistryStore: SessionRegistryApi = createSessionRegistry();
const sessionBuffers = new Map<string, SessionBufferApi>();
const sessionMetaCache = new Map<string, SessionMetaSnapshot>();
const sessionTimelineCache = new Map<
  string,
  { sourceFrames: SessionFrame[]; timeline: SessionFrame[] }
>();

function currentDaemonKey(sessionId: string): string | null {
  const daemonInstanceId = sessionRegistryStore.getSnapshot().daemonInstanceId;
  return daemonInstanceId ? `${daemonInstanceId}:${sessionId}` : null;
}

function ensureBuffer(sessionId: string, executionMode: SessionListItem['executionMode'] = 'contract'): SessionBufferApi {
  const existing = sessionBuffers.get(sessionId);
  if (existing) {
    return existing;
  }
  const buffer = createSessionBuffer(executionMode);
  sessionBuffers.set(sessionId, buffer);
  return buffer;
}

function normalizeSessionFrame(frame: SessionFrame): SessionFrame {
  if (frame.kind === 'pty' && !(frame.chunk instanceof Uint8Array)) {
    return {
      ...frame,
      chunk: new Uint8Array(frame.chunk),
    };
  }
  return frame;
}

function getMetaSnapshot(sessionId: string): SessionMetaSnapshot {
  const state = sessionRegistryStore.getSnapshot();
  const key = currentDaemonKey(sessionId);
  const session = key ? state.sessions[key] ?? null : null;
  const nextSnapshot: SessionMetaSnapshot = {
    session,
    lastSeq: key ? state.lastSeqBySession[key] ?? 0 : 0,
    gapState: key ? state.gapStateBySession[key] ?? null : null,
    archiveSeq: session?.archiveSeq ?? null,
  };
  const cached = sessionMetaCache.get(sessionId);
  if (
    cached &&
    cached.session === nextSnapshot.session &&
    cached.lastSeq === nextSnapshot.lastSeq &&
    cached.archiveSeq === nextSnapshot.archiveSeq &&
    cached.gapState?.fromSeq === nextSnapshot.gapState?.fromSeq &&
    cached.gapState?.toSeq === nextSnapshot.gapState?.toSeq
  ) {
    return cached;
  }
  sessionMetaCache.set(sessionId, nextSnapshot);
  return nextSnapshot;
}

export function resetSessionDaemonState(instanceId: string | null = null): void {
  sessionBuffers.clear();
  sessionMetaCache.clear();
  sessionTimelineCache.clear();
  if (instanceId !== null) {
    sessionRegistryStore.resetForDaemon(instanceId);
  }
}

export function hydrateSessions(items: SessionListItem[]): void {
  if (items.length === 0) {
    return;
  }
  const daemonInstanceId = items[0]?.daemonInstanceId ?? null;
  if (daemonInstanceId && sessionRegistryStore.getSnapshot().daemonInstanceId !== daemonInstanceId) {
    sessionBuffers.clear();
    sessionMetaCache.clear();
    sessionRegistryStore.resetForDaemon(daemonInstanceId);
  }
  items.forEach((session) => {
    sessionRegistryStore.applySessionMeta(session);
    ensureBuffer(session.sessionId, session.executionMode);
  });
}

export function applySessionFrame(sessionId: string, frame: SessionFrame): void {
  const meta = getMetaSnapshot(sessionId).session;
  const executionMode = meta?.executionMode ?? 'contract';
  const normalized = normalizeSessionFrame(frame);
  ensureBuffer(sessionId, executionMode).appendFrame(normalized);
  sessionRegistryStore.applyStreamFrame(sessionId, normalized);
}

export function resolveSessionReplay(
  sessionId: string,
  fromSeq: number,
  toSeq: number,
  frames: SessionFrame[] = [],
): void {
  const meta = getMetaSnapshot(sessionId).session;
  const executionMode = meta?.executionMode ?? 'contract';
  const buffer = ensureBuffer(sessionId, executionMode);
  if (frames.length > 0) {
    buffer.mergeReplay(frames.map(normalizeSessionFrame));
  }
  sessionRegistryStore.markReplayResolved(sessionId, fromSeq, toSeq);
}

export async function loadSessionsPage(
  filters: SessionListFilters = {},
  limit = 25,
): Promise<QueryEnvelope<KeysetPage<SessionPageItem>>> {
  const envelope = (await invoke('list_sessions', {
    limit,
    filters,
  })) as QueryEnvelope<KeysetPage<SessionPageItem>>;
  hydrateSessions(envelope.data.items);
  return envelope;
}

export async function retrySessionJob(jobId: string): Promise<MutationAck> {
  return (await invoke('retry_job', { jobId })) as MutationAck;
}

<<<<<<< HEAD
function adaptHistoryFrame(frame: SessionHistoryFrame): SessionFrame {
  if (frame.kind === 'pty') {
    return {
      kind: 'pty',
      seq: frame.seq,
      chunk: new TextEncoder().encode(frame.text),
      ts: new Date().toISOString(),
    };
  }

  return {
    kind: frame.kind,
    seq: frame.seq,
    chunk: frame.text,
    ts: new Date().toISOString(),
  };
}

export async function loadSessionHistory(
  request: SessionHistoryRequest,
): Promise<QueryEnvelope<SessionHistoryChunk>> {
  const envelope = (await invoke('get_session_history', {
    sessionId: request.sessionId,
    fromSeq: request.fromSeq,
    toSeq: request.toSeq,
    maxBytes: request.maxBytes ?? null,
  })) as QueryEnvelope<SessionHistoryChunk>;

  const replayFrames = envelope.data.frames.map(adaptHistoryFrame);
  const meta = getMetaSnapshot(request.sessionId);
  const executionMode = meta.session?.executionMode ?? 'contract';
  ensureBuffer(request.sessionId, executionMode).mergeReplay(replayFrames);
  if (meta.gapState) {
    resolveSessionReplay(
      request.sessionId,
      meta.gapState.fromSeq,
      meta.gapState.toSeq,
      replayFrames,
    );
  }
  return envelope;
=======
export async function loadSessionHistory(
  request: SessionHistoryRequest,
): Promise<QueryEnvelope<SessionHistoryChunk>> {
  return (await invoke('get_session_history', request)) as QueryEnvelope<SessionHistoryChunk>;
>>>>>>> 73a88b6 (testproject1-dr6.22: add session viewer and replay flow)
}

export async function stopSession(
  sessionId: string,
  mode: 'graceful' | 'force',
): Promise<MutationAck> {
<<<<<<< HEAD
  return (await invoke('stop_session', {
    sessionId,
    mode,
  })) as MutationAck;
=======
  return (await invoke('stop_session', { sessionId, mode })) as MutationAck;
>>>>>>> 73a88b6 (testproject1-dr6.22: add session viewer and replay flow)
}

export async function sendSessionInput(
  sessionId: string,
  inputUtf8: string,
): Promise<MutationAck> {
  return (await invoke('send_session_input', {
    sessionId,
    inputUtf8,
  })) as MutationAck;
}

export async function resizeSession(
  sessionId: string,
  cols: number,
  rows: number,
): Promise<MutationAck> {
  return (await invoke('resize_session', {
    sessionId,
    cols,
    rows,
  })) as MutationAck;
}

export function useSessionMeta(sessionId: string): SessionMetaSnapshot {
  return useSyncExternalStore(
    sessionRegistryStore.subscribe,
    () => getMetaSnapshot(sessionId),
    () => getMetaSnapshot(sessionId),
  );
}

export function useSessionTail(sessionId: string, maxBytes = 512): string {
  const meta = useSessionMeta(sessionId);
  const executionMode = meta.session?.executionMode ?? 'contract';
  const buffer = ensureBuffer(sessionId, executionMode);
  return useSyncExternalStore(
    buffer.subscribe,
    () => buffer.getTailText(maxBytes),
    () => buffer.getTailText(maxBytes),
  );
}

export function useSessionTimeline(sessionId: string): SessionFrame[] {
  const meta = useSessionMeta(sessionId);
  const executionMode = meta.session?.executionMode ?? 'contract';
  const buffer = ensureBuffer(sessionId, executionMode);
  const getTimelineSnapshot = (): SessionFrame[] => {
    const frames = buffer.getSnapshot().frames;
    const cached = sessionTimelineCache.get(sessionId);
    if (cached && cached.sourceFrames === frames) {
      return cached.timeline;
    }
    const timeline = frames.filter((frame) => frame.kind !== 'pty');
    sessionTimelineCache.set(sessionId, {
      sourceFrames: frames,
      timeline,
    });
    return timeline;
  };
  return useSyncExternalStore(
    buffer.subscribe,
<<<<<<< HEAD
    () => buffer.getSnapshot().frames,
    () => buffer.getSnapshot().frames,
=======
    getTimelineSnapshot,
    getTimelineSnapshot,
>>>>>>> 73a88b6 (testproject1-dr6.22: add session viewer and replay flow)
  );
}
