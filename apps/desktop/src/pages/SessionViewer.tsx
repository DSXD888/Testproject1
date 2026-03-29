import { useEffect, useMemo, useRef, useState } from 'react';
import type { ReactElement } from 'react';

import { boundUntrustedText, sanitizeUntrustedText } from '../../../../packages/ui';

import {
  loadSessionHistory,
  loadSessionsPage,
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
import { controlPlaneStore } from '../hooks/use-control-plane';
import {
  coerceBridgeError,
  parseBridgeError,
  type BridgeErrorPayload,
} from '../lib/transport';

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

const OSC8_OPEN_PATTERN = /\u001B\]8;[^;]*;[^\u0007\u001B]*(?:\u0007|\u001B\\)/g;
const OSC8_CLOSE_PATTERN = /\u001B\]8;;(?:\u0007|\u001B\\)/g;
const OSC52_PATTERN = /\u001B\]52;[^\u0007\u001B]*(?:\u0007|\u001B\\)/g;
const LIVE_TAIL_RENDER_LIMIT = 3_000;
const TRANSCRIPT_LINE_RENDER_LIMIT = 2_500;
const MAX_TRANSCRIPT_LINES = 120;

function sanitizeSessionText(text: string): string {
  return sanitizeUntrustedText(
    text
      .replace(OSC8_OPEN_PATTERN, '')
      .replace(OSC8_CLOSE_PATTERN, '')
      .replace(OSC52_PATTERN, ''),
  );
}

function previewSessionText(
  text: string,
  maxChars: number,
): ReturnType<typeof boundUntrustedText> {
  return boundUntrustedText(sanitizeSessionText(text), maxChars, 80);
}

function describeCommandFailure(action: string, error: BridgeErrorPayload): string {
  switch (error.code) {
    case 'transport_unavailable':
      return `${action} failed because the daemon bridge is unavailable. Desktop switched to browse-only mode.`;
    case 'invalid_params':
      return `${action} rejected (invalid_params): ${error.message}`;
    case 'already_terminal':
      return `${action} was ignored because the session is already terminal.`;
    case 'conflict':
      return `${action} conflicted: ${error.message}`;
    case 'busy':
      return `${action} is waiting for the daemon to clear a busy state: ${error.message}`;
    default:
      return `${action} failed (${error.code}): ${error.message}`;
  }
}

function degradeToBrowseOnly(message: string): void {
  const current = controlPlaneStore.getSnapshot().state;
  controlPlaneStore.setState({
    ...current,
    daemon: 'unavailable',
    control: 'read_only',
    reasons: [message],
  });
}

function frameText(frame: ReturnType<typeof useSessionTimeline>[number]): string {
  if (frame.kind === 'event') {
    return sanitizeUntrustedText(JSON.stringify(frame.event));
  }
  if (frame.kind === 'gap') {
    return `gap: ${frame.fromSeq}..${frame.toSeq}`;
  }
  if (frame.kind === 'lifecycle') {
    return `lifecycle: ${sanitizeUntrustedText(frame.state)}`;
  }
  return `${frame.kind.toUpperCase()}: ${previewSessionText(String(frame.chunk), TRANSCRIPT_LINE_RENDER_LIMIT).safeText}`;
}

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
  const [history, setHistory] = useState<SessionHistoryChunk | null>(null);
  const [historyState, setHistoryState] = useState<HistoryState>({ status: 'idle' });
  const [commandMessage, setCommandMessage] = useState<string | null>(null);
  const [replayMessage, setReplayMessage] = useState<string | null>(null);
  const [liveAnnouncement, setLiveAnnouncement] = useState(
    'Monitoring session state changes.',
  );
  const [inputValue, setInputValue] = useState('resume\n');
  const [inputError, setInputError] = useState<string | null>(null);
  const [interactionBlocked, setInteractionBlocked] = useState(false);
  const [retryableCommandLabel, setRetryableCommandLabel] = useState<string | null>(null);
  const retryCommandRef = useRef<(() => Promise<void>) | null>(null);
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
        const nextSession = envelope.data.items.find((item) => item.sessionId === sessionId);
        if (!nextSession) {
          setError(`Unknown session ${sessionId}`);
          return;
        }
        if (
          initialMode === undefined &&
          (nextSession.executionMode === 'contract' ||
            (nextSession.archiveSeq !== null &&
              nextSession.archiveSeq !== undefined &&
              envelope.snapshotSeq >= nextSession.archiveSeq))
        ) {
          setMode('history');
        }
      })
      .catch((loadError) => {
        if (!cancelled) {
          setError(loadError instanceof Error ? loadError.message : 'Unable to load session.');
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
        setHistory(envelope.data);
        setReplayMessage(`Replay recovered ${gapKey}.`);
        setLiveAnnouncement(`Replay recovered ${gapKey}.`);
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
        setHistory(envelope.data);
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

  const authChallenge = useMemo(() => {
    const frame = timeline.find(
      (candidate) =>
        candidate.kind === 'event' && candidate.event.type === 'authChallenge',
    );
    return frame?.kind === 'event' && frame.event.type === 'authChallenge'
      ? frame.event
      : null;
  }, [timeline]);

  const transcriptLines = useMemo(() => {
    return timeline.slice(-MAX_TRANSCRIPT_LINES).map((frame) => {
      if (frame.kind === 'stdout' || frame.kind === 'stderr') {
        const preview = previewSessionText(String(frame.chunk), TRANSCRIPT_LINE_RENDER_LIMIT);
        return `${frame.kind.toUpperCase()}: ${preview.safeText}`;
      }
      if (frame.kind === 'gap') {
        return `gap: ${frame.fromSeq}..${frame.toSeq}`;
      }
      if (frame.kind === 'event') {
        const preview = previewSessionText(
          JSON.stringify(frame.event),
          TRANSCRIPT_LINE_RENDER_LIMIT,
        );
        return `${frame.event.type}: ${preview.safeText}`;
      }
      if (frame.kind === 'lifecycle') {
        return `lifecycle: ${sanitizeUntrustedText(frame.state)}`;
      }
      const preview = previewSessionText(String(frame.chunk), TRANSCRIPT_LINE_RENDER_LIMIT);
      return `pty: ${preview.safeText}`;
    });
  }, [timeline]);
  const historyTranscript = useMemo(() => {
    const frames = history?.frames.slice(-MAX_TRANSCRIPT_LINES) ?? [];
    let truncated = false;
    const lines = frames.map((frame) => {
      const preview = previewSessionText(frameText(frame), TRANSCRIPT_LINE_RENDER_LIMIT);
      truncated = truncated || preview.truncated;
      return `${frame.kind.toUpperCase()}: ${preview.safeText}`;
    });
    return {
      lines,
      truncated,
      frameCount: history?.frames.length ?? 0,
    };
  }, [history]);

  const liveTailPreview = useMemo(
    () => boundUntrustedText(sanitizeSessionText(tail), LIVE_TAIL_RENDER_LIMIT, 120),
    [tail],
  );

  const canInteract =
    session?.executionMode === 'pty' &&
    session.reattachable &&
    session.phase === 'active' &&
    !interactionBlocked;
  const terminalControlsDisabled =
    interactionBlocked ||
    session?.phase === 'exited' ||
    session?.phase === 'reaped' ||
    session?.phase === 'archived';

  async function awaitVisible(ack: MutationAck): Promise<void> {
    let envelope = await loadSessions({}, 50);
    let attempts = 0;
    while (envelope.snapshotSeq < ack.visibleAfterSeq && attempts < 5) {
      attempts += 1;
      envelope = await loadSessions({}, 50);
    }
    setSnapshotSeq(envelope.snapshotSeq);
  }

  async function runSessionCommand(
    action: string,
    runner: () => Promise<MutationAck>,
    onSuccess: (ack: MutationAck) => Promise<void>,
    options?: { field?: 'input' },
  ): Promise<void> {
    setCommandMessage(null);
    setInputError(null);
    setRetryableCommandLabel(null);
    retryCommandRef.current = null;

    try {
      const result = (await runner()) as MutationAck | BridgeErrorPayload;
      const bridgeError = parseBridgeError(result);
      if (bridgeError) {
        throw bridgeError;
      }
      const ack = result as MutationAck;
      await onSuccess(ack);
      setInteractionBlocked(false);
    } catch (error) {
      const bridgeError = coerceBridgeError(error, `${action} failed`);
      if (bridgeError.code === 'transport_unavailable') {
        degradeToBrowseOnly(
          `Daemon bridge unavailable during ${action.toLowerCase()}. ${bridgeError.message}`,
        );
      }
      if (bridgeError.code === 'already_terminal') {
        setInteractionBlocked(true);
      }
      if (bridgeError.code === 'invalid_params' && options?.field === 'input') {
        setInputError(describeCommandFailure(action, bridgeError));
      }
      if (bridgeError.retryable) {
        setRetryableCommandLabel(action);
        retryCommandRef.current = () => runSessionCommand(action, runner, onSuccess, options);
      }
      setCommandMessage(describeCommandFailure(action, bridgeError));
      setLiveAnnouncement(describeCommandFailure(action, bridgeError));
    }
  }

  async function requestStop(nextMode: 'graceful' | 'force'): Promise<void> {
    await runSessionCommand(
      `Stop ${nextMode}`,
      () => stop(sessionId, nextMode),
      async (ack) => {
        await awaitVisible(ack);
        setCommandMessage(`Stop requested (${nextMode}) for ${sessionId}.`);
        setLiveAnnouncement(`Stop requested for ${sessionId}.`);
      },
    );
  }

  async function requestInput(): Promise<void> {
    await runSessionCommand(
      'Send PTY input',
      () => sendInput(sessionId, inputValue),
      async (ack) => {
        await awaitVisible(ack);
        setCommandMessage(`Sent PTY input to ${sessionId}.`);
        setLiveAnnouncement(`PTY input sent to ${sessionId}.`);
      },
      { field: 'input' },
    );
  }

  async function requestResize(): Promise<void> {
    await runSessionCommand(
      'Resize 120×40',
      () => resize(sessionId, 120, 40),
      async (ack) => {
        await awaitVisible(ack);
        setCommandMessage(`Requested 120×40 resize for ${sessionId}.`);
        setLiveAnnouncement(`Resize requested for ${sessionId}.`);
      },
    );
  }

  async function requestReplay(): Promise<void> {
    try {
      const envelope = await loadHistory({
        sessionId,
        fromSeq: meta.gapState?.fromSeq ?? 1,
        toSeq: meta.gapState?.toSeq ?? Math.max(meta.lastSeq, 1),
        maxBytes: 8192,
      });
      setHistory(envelope.data);
      setHistoryState({
        status: 'ready',
        availability: envelope.data.availability,
        firstSeq: envelope.data.firstSeq,
        lastSeq: envelope.data.lastSeq,
      });
      setReplayMessage(`Replay recovered ${envelope.data.firstSeq}:${envelope.data.lastSeq}.`);
      setLiveAnnouncement(`Replay recovered ${envelope.data.firstSeq}:${envelope.data.lastSeq}.`);
    } catch (historyError) {
      setHistoryState({
        status: 'error',
        message:
          historyError instanceof Error
            ? historyError.message
            : 'Replay request failed.',
      });
      setReplayMessage('Replay request failed.');
    }
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
          <h1>{sanitizeUntrustedText(session.skillPath)}</h1>
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
          <p>{sanitizeUntrustedText(authChallenge.detail)}</p>
          {authChallenge.intervention?.type === 'reveal_profile_path' ? (
            <p>
              Reveal profile path: {sanitizeUntrustedText(authChallenge.intervention.profilePath)}
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

      {commandMessage ? <p className="page__status">{sanitizeUntrustedText(commandMessage)}</p> : null}
      {replayMessage ? <p className="page__status">{sanitizeUntrustedText(replayMessage)}</p> : null}
      {retryableCommandLabel ? (
        <button
          onClick={() => {
            const retry = retryCommandRef.current;
            if (retry) {
              void retry();
            }
          }}
          type="button"
        >
          Retry {retryableCommandLabel}
        </button>
      ) : null}

      <div className="session-viewer-grid">
        <article className="detail-card">
          <h2>{mode === 'history' ? 'Transcript history' : 'Live terminal'}</h2>
          {mode === 'live' && session.executionMode === 'pty' ? (
            <>
              {liveTailPreview.truncated ? (
                <p className="page__status">
                  Live terminal output truncated to the latest{' '}
                  {liveTailPreview.maxChars.toLocaleString('en-GB')} safe characters.
                </p>
              ) : null}
              <pre aria-live="off" className="session-viewer__terminal">
                {tail ? liveTailPreview.safeText : 'Waiting for PTY output…'}
              </pre>
              <details>
                <summary>Screen-reader transcript</summary>
                <pre aria-live="off">
                  {tail ? liveTailPreview.safeText : 'No PTY transcript available yet.'}
                </pre>
              </details>
            </>
          ) : (
            <>
              {historyTranscript.frameCount > historyTranscript.lines.length ? (
                <p className="page__status">
                  Showing the most recent {historyTranscript.lines.length.toLocaleString('en-GB')}{' '}
                  transcript lines.
                </p>
              ) : null}
              {historyTranscript.truncated ? (
                <p className="page__status">Replay frame truncated to safe transcript bounds.</p>
              ) : null}
              <ol aria-live="off" className="session-viewer__timeline">
                {(historyTranscript.lines.length > 0 ? historyTranscript.lines : transcriptLines).map(
                  (line, index) => (
                  <li key={`${index}-${line}`}>{line}</li>
                  ),
                )}
              </ol>
            </>
          )}
        </article>

        <aside className="detail-card">
          <h2>Interventions</h2>
          <p>Controls are enabled only for attached PTY sessions.</p>
          <div className="decision-actions">
            <button
              type="button"
              disabled={terminalControlsDisabled}
              onClick={() => void requestStop('graceful')}
            >
              Stop graceful
            </button>
            <button
              type="button"
              disabled={terminalControlsDisabled}
              onClick={() => void requestStop('force')}
            >
              Stop force
            </button>
            <button type="button" disabled={!canInteract} onClick={() => void requestResize()}>
              Resize 120×40
            </button>
            <button type="button" onClick={() => void requestReplay()}>
              Request replay
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
          {inputError ? <p role="alert">{sanitizeUntrustedText(inputError)}</p> : null}
          <button
            type="button"
            disabled={!canInteract || inputValue.length === 0}
            onClick={() => void requestInput()}
          >
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
