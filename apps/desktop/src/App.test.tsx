import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import App from './App';
import { controlPlaneStore, type ControlPlaneState } from './hooks/use-control-plane';

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

describe('App shell', () => {
  it('owns the six route shapes and explicit not-found handling', () => {
    controlPlaneStore.setState(makeState({}));

    expect(renderToStaticMarkup(<App pathname="/" />)).toContain('Contracts route');
    expect(renderToStaticMarkup(<App pathname="/process/proc-1" />)).toContain(
      'Process route for proc-1',
    );
    expect(renderToStaticMarkup(<App pathname="/sessions" />)).toContain('Recent sessions');
    expect(renderToStaticMarkup(<App pathname="/sessions/session-1" />)).toContain(
      'Session route for session-1',
    );
    expect(renderToStaticMarkup(<App pathname="/errors" />)).toContain('Errors route');
    expect(renderToStaticMarkup(<App pathname="/daemon" />)).toContain('Daemon route');
    expect(renderToStaticMarkup(<App pathname="/missing" />)).toContain('Not found');
  });

  it('renders distinct shells for limited, browse-only, incompatible, and recovery-required modes', () => {
    controlPlaneStore.setState(makeState({ daemon: 'starting', reasons: ['Starting daemon'] }));
    expect(renderToStaticMarkup(<App pathname="/" />)).toContain('Daemon limited');

    controlPlaneStore.setState(makeState({ daemon: 'unavailable', reasons: ['Socket missing'] }));
    expect(renderToStaticMarkup(<App pathname="/" />)).toContain('read-only local snapshot');

    controlPlaneStore.setState(
      makeState({ daemon: 'incompatible', reasons: ['Protocol mismatch'] }),
    );
    expect(renderToStaticMarkup(<App pathname="/" />)).toContain('Protocol or schema mismatch');

    controlPlaneStore.setState(makeState({ storage: 'unreadable', reasons: ['Repair storage'] }));
    expect(renderToStaticMarkup(<App pathname="/" />)).toContain('Repair or recover storage');

    controlPlaneStore.reset();
  });
});
