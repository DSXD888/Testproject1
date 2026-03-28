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
  ensureBuffer(sessionId, executionMode).appendFrame(frame);
  sessionRegistryStore.applyStreamFrame(sessionId, frame);
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
    buffer.mergeReplay(frames);
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
  return useSyncExternalStore(
    buffer.subscribe,
    () => buffer.getSnapshot().frames.filter((frame) => frame.kind !== 'pty'),
    () => buffer.getSnapshot().frames.filter((frame) => frame.kind !== 'pty'),
  );
}
