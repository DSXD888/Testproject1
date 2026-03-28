import { useSyncExternalStore } from 'react';

export type ControlPlaneState = {
  daemonInstanceId: string;
  daemonVersion: string;
  protocolVersion: number;
  schemaEpoch: number;
  storage: 'ready' | 'repairing' | 'unreadable' | 'schema_mismatch';
  daemon: 'starting' | 'ready' | 'degraded' | 'unavailable' | 'incompatible';
  control: 'full' | 'limited' | 'read_only';
  load: 'normal' | 'busy' | 'saturated';
  reasons: string[];
};

export type RuntimeMode =
  | 'full control'
  | 'control limited'
  | 'browse-only'
  | 'incompatible'
  | 'recovery required';

export type ControlPlaneSnapshot = {
  state: ControlPlaneState;
  mode: RuntimeMode;
  mutationsEnabled: boolean;
};

export interface ControlPlaneStoreApi {
  getSnapshot(): ControlPlaneSnapshot;
  subscribe(onStoreChange: () => void): () => void;
  setState(nextState: ControlPlaneState): void;
  reset(): void;
}

const DEFAULT_CONTROL_PLANE_STATE: ControlPlaneState = {
  daemonInstanceId: 'pending',
  daemonVersion: 'unknown',
  protocolVersion: 1,
  schemaEpoch: 3,
  storage: 'ready',
  daemon: 'starting',
  control: 'limited',
  load: 'normal',
  reasons: ['Bootstrapping desktop control plane.'],
};

export function deriveRuntimeMode(state: ControlPlaneState): RuntimeMode {
  if (state.storage === 'unreadable' || state.storage === 'repairing') {
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

function createControlPlaneSnapshot(state: ControlPlaneState): ControlPlaneSnapshot {
  const mode = deriveRuntimeMode(state);
  return {
    state,
    mode,
    mutationsEnabled: mode === 'full control',
  };
}

function createControlPlaneStore(
  initialState: ControlPlaneState = DEFAULT_CONTROL_PLANE_STATE,
): ControlPlaneStoreApi {
  let snapshot = createControlPlaneSnapshot(initialState);
  const subscribers = new Set<() => void>();

  return {
    getSnapshot: () => snapshot,
    subscribe: (onStoreChange) => {
      subscribers.add(onStoreChange);
      return () => subscribers.delete(onStoreChange);
    },
    setState: (nextState) => {
      snapshot = createControlPlaneSnapshot(nextState);
      subscribers.forEach((subscriber) => subscriber());
    },
    reset: () => {
      snapshot = createControlPlaneSnapshot(DEFAULT_CONTROL_PLANE_STATE);
      subscribers.forEach((subscriber) => subscriber());
    },
  };
}

export const controlPlaneStore = createControlPlaneStore();

export function useControlPlaneState(): ControlPlaneSnapshot {
  return useSyncExternalStore(
    controlPlaneStore.subscribe,
    controlPlaneStore.getSnapshot,
    controlPlaneStore.getSnapshot,
  );
}
