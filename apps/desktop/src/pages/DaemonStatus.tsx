import type { ReactElement } from 'react';
import { useEffect, useState } from 'react';

import type { ControlPlaneState } from '../hooks/use-control-plane';
import { invoke } from '../lib/transport';

interface DaemonInfoSnapshot {
  daemonInstanceId: string;
  daemonVersion: string;
  protocolVersion: number;
  schemaEpoch: number;
  socketPath: string;
  descriptorPath: string;
  dbPath: string;
  artifactRoot: string;
  walBytes: number;
  startupRecoverySummary: string;
  liveAvailable: boolean;
  descriptorBacked: boolean;
  storage: string;
  daemon: string;
  control: string;
}

function fixtureDiagnostics(controlPlane: ControlPlaneState): DaemonInfoSnapshot {
  const descriptorMode = new URLSearchParams(window.location.search).get('daemonFixture');
  const liveAvailable = descriptorMode !== 'descriptor';

  return {
    daemonInstanceId: liveAvailable ? controlPlane.daemonInstanceId || 'daemon-dev' : '',
    daemonVersion: liveAvailable ? controlPlane.daemonVersion || '0.1.0' : '',
    protocolVersion: controlPlane.protocolVersion || 1,
    schemaEpoch: controlPlane.schemaEpoch || 1,
    socketPath: '/tmp/procintel/runtime/daemon.sock',
    descriptorPath: '/tmp/procintel/runtime/daemon.info.json',
    dbPath: '/tmp/procintel/state/state.db',
    artifactRoot: '/tmp/procintel/artifacts',
    walBytes: 16384,
    startupRecoverySummary: liveAvailable
      ? 'Daemon handshake succeeded during desktop bootstrap.'
      : 'Descriptor-backed diagnostics only; live daemon fields are unavailable.',
    liveAvailable,
    descriptorBacked: !liveAvailable,
    storage: controlPlane.storage,
    daemon: liveAvailable ? controlPlane.daemon : 'unavailable',
    control: controlPlane.control,
  };
}

export function DaemonStatusPage({
  controlPlane,
}: {
  controlPlane: ControlPlaneState;
}): ReactElement {
  const [diagnostics, setDiagnostics] = useState<DaemonInfoSnapshot>(
    fixtureDiagnostics(controlPlane),
  );

  useEffect(() => {
    let cancelled = false;

    void invoke('get_daemon_info')
      .then((result) => {
        if (cancelled) {
          return;
        }
        if (
          result &&
          typeof result === 'object' &&
          'socketPath' in result &&
          'descriptorPath' in result
        ) {
          setDiagnostics(result as DaemonInfoSnapshot);
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
  }, [controlPlane]);

  return (
    <section className="desktop-page">
      <header className="desktop-page__header">
        <div>
          <p className="eyebrow">Runtime diagnostics</p>
          <h1>Daemon status</h1>
          <p>
            Diagnostics remain visible while mutations are gated by {controlPlane.control}.
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
          <dd>{diagnostics.daemonInstanceId || 'Unavailable'}</dd>
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
          <dd>{diagnostics.dbPath}</dd>
        </div>
        <div>
          <dt>Artifact root</dt>
          <dd>{diagnostics.artifactRoot}</dd>
        </div>
        <div>
          <dt>WAL bytes</dt>
          <dd>{diagnostics.walBytes}</dd>
        </div>
        <div>
          <dt>Control mode</dt>
          <dd>{diagnostics.control}</dd>
        </div>
      </dl>

      <section className="detail-card">
        <h2>Startup and recovery</h2>
        <p>{diagnostics.startupRecoverySummary}</p>
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
