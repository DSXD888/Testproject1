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

interface BootstrapSnapshot {
  controlPlane: ControlPlaneState;
}

function humanizeControl(control: ControlPlaneState['control']): string {
  return control.replace('_', ' ');
}

function placeholderDiagnostics(controlPlane: ControlPlaneState): DaemonInfoSnapshot {
  return {
    instanceId: controlPlane.daemonInstanceId,
    daemonVersion: controlPlane.daemonVersion,
    protocolVersion: controlPlane.protocolVersion || 1,
    schemaEpoch: controlPlane.schemaEpoch || 1,
    socketPath: 'Unavailable',
    descriptorPath: 'Unavailable',
    stateRoot: 'Unavailable',
    stateDbPath: 'Unavailable',
    artifactRoot: 'Unavailable',
    walBytes: null,
    startedAt: '',
    startupRecovery: {
      recoveredInterruptedSessions: 0,
      lastRecoveryAt: null,
    },
    liveAvailable: false,
    descriptorBacked: true,
    storage: controlPlane.storage,
    daemon: controlPlane.daemon,
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
    liveAvailable?: boolean;
    descriptorBacked?: boolean;
    storage?: string;
    daemon?: string;
    control?: string;
  };

  return {
    ...payload,
    liveAvailable:
      typeof payload.liveAvailable === 'boolean' ? payload.liveAvailable : true,
    descriptorBacked:
      typeof payload.descriptorBacked === 'boolean'
        ? payload.descriptorBacked
        : false,
    storage: typeof payload.storage === 'string' ? payload.storage : controlPlane.storage,
    daemon: typeof payload.daemon === 'string' ? payload.daemon : controlPlane.daemon,
    control: typeof payload.control === 'string' ? payload.control : controlPlane.control,
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

function restartAllowed(
  controlPlane: ControlPlaneState,
  diagnostics: DaemonInfoSnapshot,
): boolean {
  return (
    controlPlane.storage === 'ready' &&
    !diagnostics.descriptorBacked &&
    diagnostics.daemon !== 'incompatible' &&
    (controlPlane.control === 'full' || controlPlane.control === 'limited')
  );
}

export function DaemonStatusPage({
  controlPlane,
}: {
  controlPlane: ControlPlaneState;
}): ReactElement {
  const [diagnostics, setDiagnostics] = useState<DaemonInfoSnapshot>(
    placeholderDiagnostics(controlPlane),
  );
  const [actionPending, setActionPending] = useState<
    'restart' | 'logs' | 'artifacts' | 'quit' | 'stop_quit' | null
  >(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

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
        setDiagnostics(placeholderDiagnostics(controlPlane));
      })
      .catch(() => {
        if (!cancelled) {
          setDiagnostics(placeholderDiagnostics(controlPlane));
        }
      });

    return () => {
      cancelled = true;
    };
  }, [controlPlane]);

  const canRestart = restartAllowed(controlPlane, diagnostics);

  async function handleRestart(): Promise<void> {
    setActionPending('restart');
    setActionMessage(null);
    try {
      const result = (await invoke('restart_daemon')) as {
        accepted: boolean;
        visibleAfterSeq: number;
      };
      const refreshedBootstrap = (await invoke('refresh_bootstrap_snapshot')) as BootstrapSnapshot;
      const refreshedDiagnostics = await invoke('get_daemon_info');
      const normalized =
        normalizeDiagnostics(refreshedDiagnostics, refreshedBootstrap.controlPlane) ??
        placeholderDiagnostics(refreshedBootstrap.controlPlane);
      const previousInstanceId = diagnostics.instanceId;
      setDiagnostics(normalized);
      setActionMessage(
        previousInstanceId &&
          normalized.instanceId &&
          previousInstanceId !== normalized.instanceId
          ? `Daemon restarted. Instance changed from ${previousInstanceId} to ${normalized.instanceId}.`
          : `Restart accepted at visibility fence ${result.visibleAfterSeq}.`,
      );
    } catch (error) {
      setActionMessage(error instanceof Error ? error.message : 'restart_daemon failed');
    } finally {
      setActionPending(null);
    }
  }

  async function handleOpen(kind: 'logs' | 'artifacts'): Promise<void> {
    setActionPending(kind);
    setActionMessage(null);
    try {
      const command = kind === 'logs' ? 'open_logs' : 'open_artifacts';
      const openedPath = (await invoke(command)) as string;
      setActionMessage(`Opened ${kind} at ${openedPath}.`);
    } catch (error) {
      setActionMessage(error instanceof Error ? error.message : `open_${kind} failed`);
    } finally {
      setActionPending(null);
    }
  }

  async function handleQuit(command: 'quit_ui' | 'stop_background_daemon_and_quit'): Promise<void> {
    setActionPending(command === 'quit_ui' ? 'quit' : 'stop_quit');
    setActionMessage(null);
    try {
      const result = (await invoke(command)) as string;
      setActionMessage(result);
    } catch (error) {
      setActionMessage(error instanceof Error ? error.message : `${command} failed`);
    } finally {
      setActionPending(null);
    }
  }

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
      {actionMessage ? <p role="status">{actionMessage}</p> : null}

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
        <button
          type="button"
          disabled={actionPending !== null}
          onClick={() => {
            void handleQuit('quit_ui');
          }}
        >
          Quit UI
        </button>
        {diagnostics.liveAvailable ? (
          <button
            type="button"
            disabled={actionPending !== null}
            onClick={() => {
              void handleQuit('stop_background_daemon_and_quit');
            }}
          >
            Stop background daemon and quit
          </button>
        ) : null}
        {canRestart ? (
          <button
            type="button"
            disabled={actionPending !== null}
            onClick={() => {
              void handleRestart();
            }}
          >
            Restart daemon
          </button>
        ) : null}
        <button
          type="button"
          disabled={actionPending !== null}
          onClick={() => {
            void handleOpen('logs');
          }}
        >
          Open logs
        </button>
        <button
          type="button"
          disabled={actionPending !== null}
          onClick={() => {
            void handleOpen('artifacts');
          }}
        >
          Open artifacts
        </button>
      </div>
    </section>
  );
}
