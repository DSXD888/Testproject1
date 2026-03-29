export type CleanupVerdict = 'clean' | 'forced' | 'leaked' | null;

export type StructuredSessionEvent =
  | { type: 'started'; pid: number; pgid: number; sid: number; mode: 'pty' | 'contract' }
  | { type: 'checkpoint'; checkpointSeq: number; path: string }
  | { type: 'progress'; completed: number; total?: number }
  | {
      type: 'authChallenge';
      kind: string;
      detail: string;
      intervention?: {
        type: 'reveal_profile_path';
        profilePath: string;
        reattachable: boolean;
      };
    }
  | { type: 'warning'; code: string; detail: string }
  | { type: 'finalResultReady'; path: string }
  | { type: 'exited'; exitCode: number | null };

export type SessionFrame =
  | { seq: number; kind: 'lifecycle'; state: string; cleanupVerdict?: 'clean' | 'forced' | 'leaked'; archiveSeq?: number; ts: string }
  | { seq: number; kind: 'pty'; chunk: Uint8Array; ts: string }
  | { seq: number; kind: 'stdout'; chunk: string; ts: string }
  | { seq: number; kind: 'stderr'; chunk: string; ts: string }
  | { seq: number; kind: 'event'; event: StructuredSessionEvent; ts: string }
  | { kind: 'gap'; fromSeq: number; toSeq: number };

export interface SessionListItem {
  sessionId: string;
  daemonInstanceId: string;
  skillPath: string;
  skillVersion: string | null;
  executionMode: 'pty' | 'contract';
  phase: 'created' | 'spawned' | 'active' | 'terminating' | 'exited' | 'reaped' | 'archived';
  startedAt: string | null;
  endedAt: string | null;
  durationMs: number | null;
  cleanupVerdict: CleanupVerdict;
  errorType: string | null;
  exitCode: number | null;
  checkpointCount: number;
  byteCount: number;
  reattachable: boolean;
  archiveSeq?: number | null;
}

export interface SessionRegistryState {
  daemonInstanceId: string | null;
  sessions: Record<string, SessionListItem>;
  lastSeqBySession: Record<string, number>;
  gapStateBySession: Record<string, { fromSeq: number; toSeq: number } | null>;
}

export interface SessionRegistryApi {
  getSnapshot(): SessionRegistryState;
  subscribe(onStoreChange: () => void): () => void;
  resetForDaemon(instanceId: string): void;
  applySessionMeta(session: SessionListItem): void;
  applyStreamFrame(sessionId: string, frame: SessionFrame): void;
  markReplayResolved(sessionId: string, fromSeq: number, toSeq: number): void;
  canPromoteHistorical(sessionId: string, snapshotSeq: number): boolean;
}

type AnchorStorage = Pick<Storage, 'getItem' | 'setItem'>;

const ANCHOR_STORAGE_KEY = 'procintel.sessionAnchors.v1';
const inMemoryAnchors = new Map<string, number>();

function defaultAnchorStorage(): AnchorStorage | null {
  if (typeof globalThis === 'undefined' || !('localStorage' in globalThis)) {
    return null;
  }

  const storage = globalThis.localStorage;
  if (!storage) {
    return null;
  }

  try {
    storage.getItem(ANCHOR_STORAGE_KEY);
    return storage;
  } catch {
    return null;
  }
}

function readPersistedAnchors(storage: AnchorStorage | null): Map<string, number> {
  if (!storage) {
    return new Map(inMemoryAnchors);
  }

  try {
    const raw = storage.getItem(ANCHOR_STORAGE_KEY);
    if (!raw) {
      return new Map();
    }

    const parsed = JSON.parse(raw) as Record<string, number>;
    return new Map(
      Object.entries(parsed).filter(
        (entry): entry is [string, number] => typeof entry[1] === 'number',
      ),
    );
  } catch {
    return new Map();
  }
}

function writePersistedAnchors(
  storage: AnchorStorage | null,
  anchors: Map<string, number>,
): void {
  if (!storage) {
    inMemoryAnchors.clear();
    anchors.forEach((value, key) => inMemoryAnchors.set(key, value));
    return;
  }

  try {
    storage.setItem(
      ANCHOR_STORAGE_KEY,
      JSON.stringify(Object.fromEntries(anchors.entries())),
    );
  } catch {
    // Ignore persistence failures and keep runtime memory authoritative.
  }
}

function sessionKey(daemonInstanceId: string, sessionId: string): string {
  return `${daemonInstanceId}:${sessionId}`;
}

function cloneState(state: SessionRegistryState): SessionRegistryState {
  return {
    daemonInstanceId: state.daemonInstanceId,
    sessions: { ...state.sessions },
    lastSeqBySession: { ...state.lastSeqBySession },
    gapStateBySession: { ...state.gapStateBySession },
  };
}

function hasSequence(frame: SessionFrame): frame is Exclude<SessionFrame, { kind: 'gap'; fromSeq: number; toSeq: number }> {
  return frame.kind !== 'gap';
}

export function createSessionRegistry(
  initialDaemonInstanceId: string | null = null,
  storage: AnchorStorage | null = defaultAnchorStorage(),
): SessionRegistryApi {
  const persistedAnchors = readPersistedAnchors(storage);
  let state: SessionRegistryState = {
    daemonInstanceId: initialDaemonInstanceId,
    sessions: {},
    lastSeqBySession: {},
    gapStateBySession: {},
  };
  const subscribers = new Set<() => void>();

  const emit = (): void => {
    subscribers.forEach((subscriber) => subscriber());
  };

  return {
    getSnapshot: () => state,
    subscribe: (onStoreChange) => {
      subscribers.add(onStoreChange);
      return () => subscribers.delete(onStoreChange);
    },
    resetForDaemon: (instanceId) => {
      if (state.daemonInstanceId === instanceId) {
        return;
      }

      state = {
        daemonInstanceId: instanceId,
        sessions: {},
        lastSeqBySession: {},
        gapStateBySession: {},
      };
      emit();
    },
    applySessionMeta: (session) => {
      const nextState = cloneState(state);
      nextState.daemonInstanceId = session.daemonInstanceId;
      const key = sessionKey(session.daemonInstanceId, session.sessionId);
      const prior = nextState.sessions[key];
      nextState.sessions[key] = {
        ...prior,
        ...session,
        archiveSeq: session.archiveSeq ?? prior?.archiveSeq ?? null,
      };
      if (nextState.lastSeqBySession[key] === undefined) {
        const persisted = persistedAnchors.get(key);
        if (persisted !== undefined) {
          nextState.lastSeqBySession[key] = persisted;
        }
      }
      state = nextState;
      emit();
    },
    applyStreamFrame: (sessionId, frame) => {
      const daemonInstanceId = state.daemonInstanceId;
      if (daemonInstanceId === null) {
        return;
      }

      const key = sessionKey(daemonInstanceId, sessionId);
      const nextState = cloneState(state);
      if (frame.kind === 'gap') {
        nextState.gapStateBySession[key] = {
          fromSeq: frame.fromSeq,
          toSeq: frame.toSeq,
        };
        state = nextState;
        emit();
        return;
      }

      const previousSeq = nextState.lastSeqBySession[key];
      if (previousSeq !== undefined && frame.seq <= previousSeq) {
        return;
      }

      nextState.lastSeqBySession[key] = frame.seq;
      persistedAnchors.set(key, frame.seq);
      writePersistedAnchors(storage, persistedAnchors);
      if (frame.kind === 'lifecycle' && frame.state === 'archived' && frame.archiveSeq !== undefined) {
        const session = nextState.sessions[key];
        if (session) {
          nextState.sessions[key] = {
            ...session,
            archiveSeq: frame.archiveSeq,
          };
        }
      }
      state = nextState;
      emit();
    },
    markReplayResolved: (sessionId, fromSeq, toSeq) => {
      const daemonInstanceId = state.daemonInstanceId;
      if (daemonInstanceId === null) {
        return;
      }

      const key = sessionKey(daemonInstanceId, sessionId);
      const gap = state.gapStateBySession[key];
      if (gap?.fromSeq !== fromSeq || gap?.toSeq !== toSeq) {
        return;
      }

      state = {
        ...state,
        gapStateBySession: {
          ...state.gapStateBySession,
          [key]: null,
        },
      };
      emit();
    },
    canPromoteHistorical: (sessionId, snapshotSeq) => {
      const daemonInstanceId = state.daemonInstanceId;
      if (daemonInstanceId === null) {
        return false;
      }
      const key = sessionKey(daemonInstanceId, sessionId);
      const archiveSeq = state.sessions[key]?.archiveSeq;
      return archiveSeq !== null && archiveSeq !== undefined && snapshotSeq >= archiveSeq;
    },
  };
}
