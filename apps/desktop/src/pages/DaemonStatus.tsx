import type { ReactElement } from 'react';
import { useEffect, useState } from 'react';

import type { ControlPlaneState } from '../hooks/use-control-plane';
import { invoke } from '../lib/transport';

interface DaemonInfoSnapshot {
  instanceId: string;
  daemonVersion: string;
  protocolVersion: number;
  schemaEpoch: number;
  socketPath: string;
  descriptorPath: string;
  stateRoot: string;
  stateDbPath: string;
  artifactRoot: string;
  walBytes: number | null;
  startedAt: string;
  startupRecovery: {
    recoveredInterruptedSessions: number;
    lastRecoveryAt: string | null;
  };
  liveAvailable: boolean;
  descriptorBacked: boolean;
  storage: string;
  daemon: string;
  control: string;
}

function humanizeControl(control: ControlPlaneState['control']): string {
  return control.replace('_', ' ');
}

function currentFixtureMode(): string | null {
  return new URLSearchParams(globalThis.location?.search ?? '').get('daemonFixture');
}

function fixtureDiagnostics(controlPlane: ControlPlaneState): DaemonInfoSnapshot {
  const descriptorMode = new URLSearchParams(globalThis.location?.search ?? '').get('daemonFixture');
  const liveAvailable = descriptorMode !== 'descriptor';

  return {
    instanceId: liveAvailable ? controlPlane.daemonInstanceId || 'daemon-dev' : '',
    daemonVersion: liveAvailable ? controlPlane.daemonVersion || '0.1.0' : '',
    protocolVersion: controlPlane.protocolVersion || 1,
    schemaEpoch: controlPlane.schemaEpoch || 1,
    socketPath: '/tmp/procintel/runtime/daemon.sock',
    descriptorPath: '/tmp/procintel/runtime/daemon.info.json',
    stateRoot: '/tmp/procintel/state',
    stateDbPath: '/tmp/procintel/state/state.db',
    artifactRoot: '/tmp/procintel/artifacts',
    walBytes: 16384,
    startedAt: '2026-03-28T08:00:00Z',
    startupRecovery: {
      recoveredInterruptedSessions: liveAvailable ? 1 : 0,
      lastRecoveryAt: liveAvailable ? '2026-03-28T07:58:00Z' : null,
    },
    liveAvailable,
    descriptorBacked: !liveAvailable,
    storage: controlPlane.storage,
    daemon: liveAvailable ? controlPlane.daemon : 'unavailable',
    control: controlPlane.control,
  };
}

function normalizeDiagnostics(
  result: unknown,
  controlPlane: ControlPlaneState,
): DaemonInfoSnapshot | null {
  if (
    !result ||
    typeof result !== 'object' ||
    !('instanceId' in result) ||
    !('stateDbPath' in result) ||
    !('startupRecovery' in result)
  ) {
    return null;
  }

  const payload = result as {
    instanceId: string;
    daemonVersion: string;
    protocolVersion: number;
    schemaEpoch: number;
    socketPath: string;
    descriptorPath: string;
    stateRoot: string;
    stateDbPath: string;
    artifactRoot: string;
    walBytes: number | null;
    startedAt: string;
    startupRecovery: {
      recoveredInterruptedSessions: number;
      lastRecoveryAt: string | null;
    };
  };

  return {
    ...payload,
    liveAvailable: true,
    descriptorBacked: false,
    storage: controlPlane.storage,
    daemon: controlPlane.daemon,
    control: controlPlane.control,
  };
}

function startupRecoverySummary(diagnostics: DaemonInfoSnapshot): string {
  if (!diagnostics.liveAvailable) {
    return 'Descriptor-backed diagnostics only; live daemon fields are unavailable.';
  }

  if (diagnostics.startupRecovery.recoveredInterruptedSessions === 0) {
    return 'Daemon handshake succeeded during desktop bootstrap with no interrupted sessions to recover.';
  }

  const recovered = diagnostics.startupRecovery.recoveredInterruptedSessions;
  const lastRecoveryAt = diagnostics.startupRecovery.lastRecoveryAt ?? 'unknown time';
  return `Recovered ${recovered} interrupted session(s); last recovery at ${lastRecoveryAt}.`;
}

export function DaemonStatusPage({
  controlPlane,
}: {
  controlPlane: ControlPlaneState;
}): ReactElement {
  const fixtureMode = currentFixtureMode();
  const [diagnostics, setDiagnostics] = useState<DaemonInfoSnapshot>(
    fixtureDiagnostics(controlPlane),
  );

  useEffect(() => {
    let cancelled = false;
    if (fixtureMode) {
      setDiagnostics(fixtureDiagnostics(controlPlane));
      return () => {
        cancelled = true;
      };
    }

    void invoke('get_daemon_info')
      .then((result) => {
        if (cancelled) {
          return;
        }
        const normalized = normalizeDiagnostics(result, controlPlane);
        if (normalized) {
          setDiagnostics(normalized);
          return;
        }
        setDiagnostics(fixtureDiagnostics(controlPlane));
      })
      .catch(() => {
        if (!cancelled) {
          setDiagnostics(fixtureDiagnostics(controlPlane));
        }
      });

    return () => {
      cancelled = true;
    };
  }, [controlPlane, fixtureMode]);

  return (
    <section className="desktop-page">
      <header className="desktop-page__header">
        <div>
          <p className="eyebrow">Runtime diagnostics</p>
          <h1>Daemon status</h1>
          <p>
            Diagnostics remain visible while mutations are gated by {humanizeControl(controlPlane.control)}.
          </p>
        </div>
        <div className="meta-card">
          <span>Transport</span>
          <strong>{diagnostics.liveAvailable ? 'live' : 'descriptor-backed'}</strong>
          <span>{diagnostics.daemon}</span>
        </div>
      </header>

      {!diagnostics.liveAvailable ? (
        <div className="banner banner--warning" role="status">
          Live daemon unavailable. Showing descriptor-backed diagnostics only.
        </div>
      ) : null}

      <dl className="diagnostics-grid">
        <div>
          <dt>Instance ID</dt>
          <dd>{diagnostics.instanceId || 'Unavailable'}</dd>
        </div>
        <div>
          <dt>Daemon version</dt>
          <dd>{diagnostics.daemonVersion || 'Unavailable'}</dd>
        </div>
        <div>
          <dt>Protocol version</dt>
          <dd>{diagnostics.protocolVersion}</dd>
        </div>
        <div>
          <dt>Schema epoch</dt>
          <dd>{diagnostics.schemaEpoch}</dd>
        </div>
        <div>
          <dt>Socket path</dt>
          <dd>{diagnostics.socketPath}</dd>
        </div>
        <div>
          <dt>Descriptor path</dt>
          <dd>{diagnostics.descriptorPath}</dd>
        </div>
        <div>
          <dt>DB path</dt>
          <dd>{diagnostics.stateDbPath}</dd>
        </div>
        <div>
          <dt>Artifact root</dt>
          <dd>{diagnostics.artifactRoot}</dd>
        </div>
        <div>
          <dt>WAL bytes</dt>
          <dd>{diagnostics.walBytes ?? 'Unavailable'}</dd>
        </div>
        <div>
          <dt>Control mode</dt>
          <dd>{diagnostics.control}</dd>
        </div>
      </dl>

      <section className="detail-card">
        <h2>Startup and recovery</h2>
        <p>{startupRecoverySummary(diagnostics)}</p>
      </section>

      <div className="action-row">
        <button type="button" disabled>
          Restart daemon
        </button>
        <button type="button" disabled>
          Open logs
        </button>
        <button type="button" disabled>
          Open artifacts
        </button>
      </div>
    </section>
  );
}
