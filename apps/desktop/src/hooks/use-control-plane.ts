import { useEffect, useSyncExternalStore } from 'react';

import { invoke } from '../lib/transport';

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

const listeners = new Set<() => void>();

const defaultSnapshot = fixtureSnapshot();

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

async function refreshSnapshot(): Promise<void> {
  state.loading = true;
  emitChange();

  try {
    const result = await invoke('get_bootstrap_snapshot');
    const snapshot = parseBootstrapResult(result);
    state.snapshot = snapshot ?? fixtureSnapshot();
    state.error = snapshot ? null : 'Desktop bridge unavailable; using local fixture.';
  } catch (error) {
    state.snapshot = fixtureSnapshot();
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

  if (
    result &&
    typeof result === 'object' &&
    'code' in result &&
    (result as { code?: string }).code === 'dev_bridge_unconfigured'
  ) {
    return null;
  }

  return null;
}

function fixtureSnapshot(): BootstrapSnapshot {
  const params = new URLSearchParams(window.location.search);
  const requestedMode = params.get('mode');

  switch (requestedMode) {
    case 'browse-only':
      return {
        mode: 'browse_only',
        controlPlane: {
          daemonInstanceId: '',
          daemonVersion: '',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'ready',
          daemon: 'unavailable',
          control: 'read_only',
          load: 'normal',
          reasons: ['Daemon unavailable. Local snapshot is read-only.'],
        },
        descriptor: null,
        launchedDaemon: false,
      };
    case 'control-limited':
      return {
        mode: 'control_limited',
        controlPlane: {
          daemonInstanceId: 'daemon-limited',
          daemonVersion: '0.1.0',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'ready',
          daemon: 'degraded',
          control: 'limited',
          load: 'busy',
          reasons: ['Daemon is degraded. Some actions are unavailable.'],
        },
        descriptor: null,
        launchedDaemon: false,
      };
    case 'incompatible':
      return {
        mode: 'incompatible',
        controlPlane: {
          daemonInstanceId: '',
          daemonVersion: '',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'schema_mismatch',
          daemon: 'incompatible',
          control: 'read_only',
          load: 'normal',
          reasons: ['Desktop and daemon are incompatible.'],
        },
        descriptor: null,
        launchedDaemon: false,
      };
    case 'recovery':
      return {
        mode: 'recovery_required',
        controlPlane: {
          daemonInstanceId: '',
          daemonVersion: '',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'repairing',
          daemon: 'unavailable',
          control: 'read_only',
          load: 'normal',
          reasons: ['Storage recovery is required.'],
        },
        descriptor: null,
        launchedDaemon: false,
      };
    default:
      return {
        mode: 'full_control',
        controlPlane: {
          daemonInstanceId: 'daemon-dev',
          daemonVersion: '0.1.0',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'ready',
          daemon: 'ready',
          control: 'full',
          load: 'normal',
          reasons: ['Desktop dev fixture is active.'],
        },
        descriptor: {
          instanceId: 'daemon-dev',
          pid: 1,
          socketPath: '/tmp/daemon.sock',
          protocolVersion: 1,
          schemaEpoch: 1,
          stateRoot: '/tmp/state',
          startedAt: '2026-03-28T12:00:00Z',
          ready: true,
        },
        launchedDaemon: false,
      };
  }
}

export function useControlPlaneState(): ControlPlaneStoreState {
  const snapshot = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  useEffect(() => {
    void refreshSnapshot();
  }, []);

  return snapshot;
}
