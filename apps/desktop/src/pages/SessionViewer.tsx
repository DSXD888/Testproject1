import { useEffect, useMemo, useRef, useState } from 'react';
import type { ReactElement } from 'react';

import {
  applySessionFrame,
  loadSessionHistory,
  loadSessionsPage,
  resolveSessionReplay,
  resizeSession,
  sendSessionInput,
  stopSession,
  useSessionMeta,
  useSessionTail,
  useSessionTimeline,
  type MutationAck,
  type QueryEnvelope,
  type SessionHistoryChunk,
  type SessionHistoryRequest,
} from '../hooks/use-daemon';
import { seedDevSessionFrames } from '../lib/dev-bridge-fixtures';

type ViewerMode = 'live' | 'history';
type HistoryState =
  | { status: 'idle' | 'loading' }
  | {
      status: 'ready';
      availability: SessionHistoryChunk['availability'];
      firstSeq: number;
      lastSeq: number;
    }
  | { status: 'error'; message: string };

export function SessionViewer({
  sessionId,
  initialMode,
  onBack,
  loadSessions = loadSessionsPage,
  loadHistory = loadSessionHistory,
  stop = stopSession,
  sendInput = sendSessionInput,
  resize = resizeSession,
}: {
  sessionId: string;
  initialMode?: ViewerMode;
  onBack: () => void;
  loadSessions?: typeof loadSessionsPage;
  loadHistory?: (
    request: SessionHistoryRequest,
  ) => Promise<QueryEnvelope<SessionHistoryChunk>>;
  stop?: typeof stopSession;
  sendInput?: typeof sendSessionInput;
  resize?: typeof resizeSession;
}): ReactElement {
  const meta = useSessionMeta(sessionId);
  const tail = useSessionTail(sessionId, 8192);
  const timeline = useSessionTimeline(sessionId);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [snapshotSeq, setSnapshotSeq] = useState(0);
  const [mode, setMode] = useState<ViewerMode>(initialMode ?? 'live');
  const [historyState, setHistoryState] = useState<HistoryState>({ status: 'idle' });
  const [commandMessage, setCommandMessage] = useState<string | null>(null);
  const [replayMessage, setReplayMessage] = useState<string | null>(null);
  const [liveAnnouncement, setLiveAnnouncement] = useState(
    'Monitoring session state changes.',
  );
  const [inputValue, setInputValue] = useState('resume\n');
  const seeded = useRef(false);
  const handledGap = useRef<string | null>(null);
  const session = meta.session;

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    loadSessions({}, 50)
      .then((envelope) => {
        if (cancelled) {
          return;
        }
        setSnapshotSeq(envelope.snapshotSeq);
        const session = envelope.data.items.find((item) => item.sessionId === sessionId);
        if (!session) {
          setError(`Unknown session ${sessionId}`);
          return;
        }
        if (
          initialMode === undefined &&
          (session.executionMode === 'contract' ||
            (session.archiveSeq !== null &&
              session.archiveSeq !== undefined &&
              envelope.snapshotSeq >= session.archiveSeq))
        ) {
          setMode('history');
        }
      })
      .catch((loadError) => {
        if (!cancelled) {
          setError(
            loadError instanceof Error ? loadError.message : 'Unable to load session.',
          );
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [initialMode, loadSessions, sessionId]);

  useEffect(() => {
    if (meta.session === null || seeded.current) {
      return;
    }
    seedDevSessionFrames(sessionId).forEach((frame) => applySessionFrame(sessionId, frame));
    seeded.current = true;
  }, [meta.session, sessionId]);

  useEffect(() => {
    if (
      initialMode === undefined &&
      meta.session?.phase === 'archived' &&
      meta.archiveSeq !== null &&
      snapshotSeq >= meta.archiveSeq
    ) {
      setMode('history');
    }
  }, [initialMode, meta.archiveSeq, meta.session?.phase, snapshotSeq]);

  useEffect(() => {
    const gap = meta.gapState;
    if (!gap) {
      return;
    }

    const gapKey = `${gap.fromSeq}:${gap.toSeq}`;
    if (handledGap.current === gapKey) {
      return;
    }
    handledGap.current = gapKey;
    setHistoryState({ status: 'loading' });
    setLiveAnnouncement(`Recovering output gap ${gapKey}.`);

    void loadHistory({
      sessionId,
      fromSeq: gap.fromSeq,
      toSeq: gap.toSeq,
      maxBytes: 8192,
    })
      .then((envelope) => {
        if (envelope.data.availability === 'complete') {
          resolveSessionReplay(
            sessionId,
            gap.fromSeq,
            gap.toSeq,
            envelope.data.frames,
          );
          setReplayMessage(`Replay recovered ${gapKey}.`);
          setLiveAnnouncement(`Replay recovered ${gapKey}.`);
        } else {
          envelope.data.frames.forEach((frame) => applySessionFrame(sessionId, frame));
          setReplayMessage(
            `Replay ${envelope.data.availability} for ${gapKey}.`,
          );
          setLiveAnnouncement(`Replay incomplete for ${gapKey}.`);
        }
        setHistoryState({
          status: 'ready',
          availability: envelope.data.availability,
          firstSeq: envelope.data.firstSeq,
          lastSeq: envelope.data.lastSeq,
        });
      })
      .catch((replayError) => {
        setHistoryState({
          status: 'error',
          message:
            replayError instanceof Error
              ? replayError.message
              : 'Replay request failed.',
        });
      });
  }, [loadHistory, meta.gapState, sessionId]);

  useEffect(() => {
    if (
      session === null ||
      mode !== 'history' ||
      historyState.status !== 'idle' ||
      session.executionMode !== 'contract'
    ) {
      return;
    }

    setHistoryState({ status: 'loading' });
    void loadHistory({
      sessionId,
      fromSeq: 1,
      toSeq: Math.max(meta.lastSeq, 1),
      maxBytes: 8192,
    })
      .then((envelope) => {
        envelope.data.frames.forEach((frame) => applySessionFrame(sessionId, frame));
        setHistoryState({
          status: 'ready',
          availability: envelope.data.availability,
          firstSeq: envelope.data.firstSeq,
          lastSeq: envelope.data.lastSeq,
        });
        setReplayMessage(`Loaded historical replay for ${sessionId}.`);
      })
      .catch((historyError) => {
        setHistoryState({
          status: 'error',
          message:
            historyError instanceof Error
              ? historyError.message
              : 'Historical replay request failed.',
        });
      });
  }, [historyState.status, loadHistory, meta.lastSeq, mode, session, sessionId]);

  const canInteract = session?.executionMode === 'pty' && session.reattachable;
  const authChallenge = useMemo(() => {
    const frame = timeline.find(
      (candidate) =>
        candidate.kind === 'event' && candidate.event.type === 'authChallenge',
    );
    return frame?.kind === 'event' && frame.event.type === 'authChallenge'
      ? frame.event
      : null;
  }, [timeline]);

  const transcriptLines = timeline.map((frame) => {
    if (frame.kind === 'stdout' || frame.kind === 'stderr') {
      return `${frame.kind.toUpperCase()}: ${frame.chunk}`;
    }
    if (frame.kind === 'gap') {
      return `gap: ${frame.fromSeq}..${frame.toSeq}`;
    }
    if (frame.kind === 'event') {
      return `${frame.event.type}: ${JSON.stringify(frame.event)}`;
    }
    if (frame.kind === 'lifecycle') {
      return `lifecycle: ${frame.state}`;
    }
    return 'pty frame';
  });

  async function awaitVisible(ack: MutationAck): Promise<void> {
    let envelope = await loadSessions({}, 50);
    let attempts = 0;
    while (envelope.snapshotSeq < ack.visibleAfterSeq && attempts < 5) {
      attempts += 1;
      envelope = await loadSessions({}, 50);
    }
    setSnapshotSeq(envelope.snapshotSeq);
  }

  async function requestStop(nextMode: 'graceful' | 'force'): Promise<void> {
    const ack = await stop(sessionId, nextMode);
    await awaitVisible(ack);
    setCommandMessage(`Stop requested (${nextMode}) for ${sessionId}.`);
    setLiveAnnouncement(`Stop requested for ${sessionId}.`);
  }

  async function requestInput(): Promise<void> {
    const ack = await sendInput(sessionId, inputValue);
    await awaitVisible(ack);
    setCommandMessage(`Sent PTY input to ${sessionId}.`);
    setLiveAnnouncement(`PTY input sent to ${sessionId}.`);
  }

  async function requestResize(): Promise<void> {
    const ack = await resize(sessionId, 120, 40);
    await awaitVisible(ack);
    setCommandMessage(`Requested 120×40 resize for ${sessionId}.`);
    setLiveAnnouncement(`Resize requested for ${sessionId}.`);
  }

  if (loading) {
    return (
      <section className="desktop-page">
        <div className="empty-state">Loading session viewer…</div>
      </section>
    );
  }

  if (error || session === null) {
    return (
      <section className="desktop-page">
        <button type="button" onClick={onBack}>
          Back to sessions
        </button>
        <div className="empty-state" role="alert">
          {error ?? `Unknown session ${sessionId}`}
        </div>
      </section>
    );
  }

  return (
    <section className="desktop-page">
      <span hidden>Session route for {sessionId}</span>
      <div className="sr-only" aria-live="polite">
        {liveAnnouncement}
      </div>

      <header className="desktop-page__header">
        <div>
          <button type="button" onClick={onBack}>
            Back to sessions
          </button>
          <p className="eyebrow">Session viewer</p>
          <h1>{session.skillPath}</h1>
          <p>
            {session.executionMode} · {session.phase} · snapshot {snapshotSeq}
          </p>
        </div>
        <div className="meta-card">
          <span>Viewer mode</span>
          <strong>{mode}</strong>
          <span>Last seq {meta.lastSeq}</span>
        </div>
      </header>

      {authChallenge ? (
        <div className="banner banner--warning">
          <strong>Browser auth challenge</strong>
          <p>{authChallenge.detail}</p>
          {authChallenge.intervention?.type === 'reveal_profile_path' ? (
            <p>
              Reveal profile path: {authChallenge.intervention.profilePath}
              {' · '}
              {authChallenge.intervention.reattachable
                ? 'Reattachable after sign-in.'
                : 'Session cannot be reattached.'}
            </p>
          ) : null}
        </div>
      ) : null}

      {meta.gapState ? (
        <div className="banner banner--warning" data-testid="session-gap">
          Missing output from seq {meta.gapState.fromSeq} to {meta.gapState.toSeq}.
          {historyState.status === 'ready'
            ? ` Replay availability: ${historyState.availability}.`
            : ''}
        </div>
      ) : null}

      {commandMessage ? <p className="page__status">{commandMessage}</p> : null}
      {replayMessage ? <p className="page__status">{replayMessage}</p> : null}

      <div className="session-viewer-grid">
        <article className="detail-card">
          <h2>{mode === 'history' ? 'Transcript history' : 'Live terminal'}</h2>
          {mode === 'live' && session.executionMode === 'pty' ? (
            <>
              <pre className="session-viewer__terminal">
                {tail || 'Waiting for PTY output…'}
              </pre>
              <details>
                <summary>Screen-reader transcript</summary>
                <pre>{tail || 'No PTY transcript available yet.'}</pre>
              </details>
            </>
          ) : (
            <ol className="session-viewer__timeline">
              {transcriptLines.map((line, index) => (
                <li key={`${index}-${line}`}>{line}</li>
              ))}
            </ol>
          )}
        </article>

        <aside className="detail-card">
          <h2>Interventions</h2>
          <p>Controls are enabled only for attached PTY sessions.</p>
          <div className="decision-actions">
            <button type="button" disabled={!canInteract} onClick={() => void requestStop('graceful')}>
              Stop graceful
            </button>
            <button type="button" disabled={!canInteract} onClick={() => void requestStop('force')}>
              Stop force
            </button>
            <button type="button" disabled={!canInteract} onClick={() => void requestResize()}>
              Resize 120×40
            </button>
          </div>
          <label className="session-viewer__input">
            PTY input
            <textarea
              value={inputValue}
              onChange={(event) => setInputValue(event.currentTarget.value)}
              disabled={!canInteract}
            />
          </label>
          <button type="button" disabled={!canInteract || inputValue.length === 0} onClick={() => void requestInput()}>
            Send PTY input
          </button>
          {!canInteract ? (
            <p className="page__status">
              Input is disabled for contract, detached, or archived sessions.
            </p>
          ) : null}
        </aside>
      </div>
    </section>
  );
}

export default SessionViewer;
