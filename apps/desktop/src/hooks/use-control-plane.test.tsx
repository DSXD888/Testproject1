import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import {
  controlPlaneStore,
  deriveRuntimeMode,
  type ControlPlaneState,
  useControlPlaneState,
} from './use-control-plane';

function makeState(overrides: Partial<ControlPlaneState>): ControlPlaneState {
  return {
    daemonInstanceId: 'daemon-1',
    daemonVersion: '1.0.0',
    protocolVersion: 1,
    schemaEpoch: 3,
    storage: 'ready',
    daemon: 'ready',
    control: 'full',
    load: 'normal',
    reasons: ['Healthy'],
    ...overrides,
  };
}

function Probe() {
  const snapshot = useControlPlaneState();
  return <div>{snapshot.mode}</div>;
}

describe('useControlPlaneState', () => {
  it('derives the five runtime modes with total precedence', () => {
    expect(deriveRuntimeMode(makeState({ storage: 'repairing' }))).toBe('recovery required');
    expect(deriveRuntimeMode(makeState({ storage: 'schema_mismatch' }))).toBe('incompatible');
    expect(deriveRuntimeMode(makeState({ daemon: 'unavailable' }))).toBe('browse-only');
    expect(deriveRuntimeMode(makeState({ daemon: 'degraded' }))).toBe('control limited');
    expect(deriveRuntimeMode(makeState({ control: 'limited' }))).toBe('control limited');
    expect(deriveRuntimeMode(makeState({ load: 'busy' }))).toBe('control limited');
    expect(deriveRuntimeMode(makeState({ daemon: 'ready', control: 'full', load: 'normal' }))).toBe(
      'full control',
    );
  });

  it('uses a minimal external-store API', () => {
    const observed: string[] = [];
    const unsubscribe = controlPlaneStore.subscribe(() => {
      observed.push(controlPlaneStore.getSnapshot().mode);
    });

    controlPlaneStore.setState(makeState({ daemon: 'starting' }));
    unsubscribe();

    expect(observed).toContain('control limited');
    expect(typeof controlPlaneStore.getSnapshot).toBe('function');
  });

  it('reads state via useSyncExternalStore through the hook', () => {
    controlPlaneStore.setState(makeState({ daemon: 'unavailable', reasons: ['Socket missing'] }));
    const markup = renderToStaticMarkup(<Probe />);

    expect(markup).toContain('browse-only');
    controlPlaneStore.reset();
  });
});
