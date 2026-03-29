import { useEffect, useSyncExternalStore } from 'react';

import { invoke } from '../lib/transport';

export type HumanRuntimeMode =
  | 'full control'
  | 'control limited'
  | 'browse-only'
  | 'incompatible'
  | 'recovery required';

export type RuntimeMode =
  | 'full_control'
  | 'control_limited'
  | 'browse_only'
  | 'incompatible'
  | 'recovery_required';

export interface ControlPlaneState {
  daemonInstanceId: string;
  daemonVersion: string;
  protocolVersion: number;
  schemaEpoch: number;
  storage: 'ready' | 'repairing' | 'unreadable' | 'schema_mismatch';
  daemon: 'starting' | 'ready' | 'degraded' | 'unavailable' | 'incompatible';
  control: 'full' | 'limited' | 'read_only';
  load: 'normal' | 'busy' | 'saturated';
  reasons: string[];
}

export interface BootstrapSnapshot {
  mode: RuntimeMode;
  controlPlane: ControlPlaneState;
  descriptor: {
    instanceId: string;
    pid: number;
    socketPath: string;
    protocolVersion: number;
    schemaEpoch: number;
    stateRoot: string;
    startedAt: string;
    ready: boolean;
  } | null;
  launchedDaemon: boolean;
}

type ControlPlaneStoreState = {
  snapshot: BootstrapSnapshot;
  loading: boolean;
  error: string | null;
};

export interface ControlPlaneStoreSnapshot {
  state: ControlPlaneState;
  mode: HumanRuntimeMode;
  mutationsEnabled: boolean;
}

export type UseControlPlaneStateResult = ControlPlaneStoreState & ControlPlaneStoreSnapshot;

const listeners = new Set<() => void>();

function bootstrapPendingSnapshot(): BootstrapSnapshot {
  return {
    mode: 'control_limited',
    controlPlane: {
      daemonInstanceId: '',
      daemonVersion: '',
      protocolVersion: 1,
      schemaEpoch: 1,
      storage: 'ready',
      daemon: 'starting',
      control: 'limited',
      load: 'busy',
      reasons: ['Loading desktop bootstrap.'],
    },
    descriptor: null,
    launchedDaemon: false,
  };
}

const defaultSnapshot = bootstrapPendingSnapshot();

const state: ControlPlaneStoreState = {
  snapshot: defaultSnapshot,
  loading: true,
  error: null,
};

function emitChange(): void {
  for (const listener of listeners) {
    listener();
  }
}

function getSnapshot(): ControlPlaneStoreState {
  return state;
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function shouldPreserveCurrentSnapshot(nextSnapshot: BootstrapSnapshot): boolean {
  const currentReasons = state.snapshot.controlPlane.reasons.join(' ').toLowerCase();
  return (
    state.snapshot.controlPlane.daemon === 'unavailable' &&
    state.snapshot.controlPlane.control === 'read_only' &&
    currentReasons.includes('bridge unavailable') &&
    nextSnapshot.mode === 'full_control'
  );
}

async function refreshSnapshot(): Promise<void> {
  state.loading = true;
  emitChange();

  try {
    const result = await invoke('get_bootstrap_snapshot');
    const snapshot = parseBootstrapResult(result);
    if (snapshot && !shouldPreserveCurrentSnapshot(snapshot)) {
      state.snapshot = snapshot;
    }
    state.error = snapshot ? null : 'Desktop bootstrap returned an unexpected payload.';
  } catch (error) {
    state.error = error instanceof Error ? error.message : 'Desktop bootstrap failed';
  } finally {
    state.loading = false;
    emitChange();
  }
}

function parseBootstrapResult(result: unknown): BootstrapSnapshot | null {
  if (
    result &&
    typeof result === 'object' &&
    'mode' in result &&
    'controlPlane' in result
  ) {
    return result as BootstrapSnapshot;
  }

  return null;
}

export function useControlPlaneState(): UseControlPlaneStateResult {
  const snapshot = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  useEffect(() => {
    void refreshSnapshot();
  }, []);

  return {
    ...snapshot,
    ...controlPlaneStore.getSnapshot(),
  };
}

export function deriveRuntimeMode(state: ControlPlaneState): HumanRuntimeMode {
  if (state.storage === 'repairing' || state.storage === 'unreadable') {
    return 'recovery required';
  }
  if (state.storage === 'schema_mismatch' || state.daemon === 'incompatible') {
    return 'incompatible';
  }
  if (state.daemon === 'unavailable') {
    return 'browse-only';
  }
  if (
    state.daemon === 'starting' ||
    state.daemon === 'degraded' ||
    state.control === 'limited' ||
    state.control === 'read_only' ||
    state.load === 'busy' ||
    state.load === 'saturated'
  ) {
    return 'control limited';
  }
  return 'full control';
}

function toRuntimeMode(mode: HumanRuntimeMode): RuntimeMode {
  switch (mode) {
    case 'full control':
      return 'full_control';
    case 'control limited':
      return 'control_limited';
    case 'browse-only':
      return 'browse_only';
    case 'incompatible':
      return 'incompatible';
    case 'recovery required':
      return 'recovery_required';
  }
}

export const controlPlaneStore = {
  getSnapshot(): ControlPlaneStoreSnapshot {
    const currentState = state.snapshot.controlPlane;
    const mode = deriveRuntimeMode(currentState);
    return {
      state: currentState,
      mode,
      mutationsEnabled: mode === 'full control',
    };
  },
  subscribe,
  setState(nextState: ControlPlaneState): void {
    state.snapshot = {
      ...state.snapshot,
      mode: toRuntimeMode(deriveRuntimeMode(nextState)),
      controlPlane: nextState,
    };
    state.loading = false;
    state.error = null;
    emitChange();
  },
  reset(): void {
    state.snapshot = bootstrapPendingSnapshot();
    state.loading = false;
    state.error = null;
    emitChange();
  },
};
