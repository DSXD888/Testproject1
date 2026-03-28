import { useEffect, useMemo, useState } from 'react';
import type { ReactElement } from 'react';

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
  type SessionHistoryChunk,
} from '../hooks/use-daemon';

type SessionViewerProps = {
  sessionId: string;
  onBack: () => void;
};

type ViewerMode = 'live' | 'history';

function canInteract(
  executionMode: 'pty' | 'contract' | undefined,
  phase:
    | 'created'
    | 'spawned'
    | 'active'
    | 'terminating'
    | 'exited'
    | 'reaped'
    | 'archived'
    | undefined,
  reattachable: boolean | undefined,
): boolean {
  return executionMode === 'pty' && reattachable === true && phase === 'active';
}

function formatAckMessage(action: string, ack: MutationAck): string {
  return `${action} accepted at visibility fence ${ack.visibleAfterSeq}.`;
}

function frameText(frame: ReturnType<typeof useSessionTimeline>[number]): string {
  if (frame.kind === 'event') {
    return JSON.stringify(frame.event);
  }
  if (frame.kind === 'gap') {
    return `Gap from ${frame.fromSeq} to ${frame.toSeq}`;
  }
  if (frame.kind === 'lifecycle') {
    return frame.state;
  }
  return 'chunk' in frame ? String(frame.chunk) : '';
}

function authGuidance(frames: ReturnType<typeof useSessionTimeline>): string | null {
  const authEvent = frames.find(
    (frame) => frame.kind === 'event' && frame.event.type === 'authChallenge',
  );
  if (
    authEvent &&
    authEvent.kind === 'event' &&
    authEvent.event.type === 'authChallenge'
  ) {
    return `Browser-only auth challenge detected: ${authEvent.event.detail}. Use reveal_profile_path guidance and reattach once the challenge is cleared.`;
  }
  const transcriptHint = frames.find((frame) => {
    const text = frameText(frame).toLowerCase();
    return text.includes('auth challenge') || text.includes('reveal_profile_path');
  });
  if (!transcriptHint) {
    return null;
  }
  return 'Browser-only auth challenge detected: use reveal_profile_path guidance and reattach once the challenge is cleared.';
}

export function SessionViewer({
  sessionId,
  onBack,
}: SessionViewerProps): ReactElement {
  const meta = useSessionMeta(sessionId);
  const tail = useSessionTail(sessionId, 4096);
  const timeline = useSessionTimeline(sessionId);
  const [history, setHistory] = useState<SessionHistoryChunk | null>(null);
  const [viewerMode, setViewerMode] = useState<ViewerMode>('live');
  const [commandMessage, setCommandMessage] = useState<string | null>(null);
  const [inputValue, setInputValue] = useState('reauth\n');

  useEffect(() => {
    void loadSessionsPage({}, 50);
  }, []);

  useEffect(() => {
    if (!meta.session) {
      return;
    }

    const targetSeq = Math.max(meta.lastSeq, 1);
    void loadSessionHistory({
      sessionId,
      fromSeq: 1,
      toSeq: targetSeq,
      maxBytes: 8192,
    }).then((envelope) => {
      setHistory(envelope.data);
      if (meta.session?.executionMode === 'contract') {
        setViewerMode('history');
      }
    });
  }, [meta.lastSeq, meta.session, sessionId]);

  const interactive = useMemo(
    () =>
      canInteract(
        meta.session?.executionMode,
        meta.session?.phase,
        meta.session?.reattachable,
      ),
    [meta.session],
  );

  const guidance = authGuidance(timeline);

  return (
    <section className="page page--detail">
      <header className="page__header">
        <div>
          <button onClick={onBack} type="button">
            Back to sessions
          </button>
          <p className="page__eyebrow">Session viewer</p>
          <h2>Session viewer</h2>
          <p>{sessionId}</p>
          <p className="page__lede">
            Execution mode: {meta.session?.executionMode ?? 'loading'} • phase:{' '}
            {meta.session?.phase ?? 'loading'}
          </p>
        </div>
        <div className="decision-actions">
          <button
            data-selected={viewerMode === 'live'}
            onClick={() => setViewerMode('live')}
            type="button"
          >
            Live terminal
          </button>
          <button
            data-selected={viewerMode === 'history'}
            onClick={() => setViewerMode('history')}
            type="button"
          >
            History
          </button>
        </div>
      </header>

      <section className="detail-grid">
        <article className="detail-card">
          <h3>Session status</h3>
          <p>Last sequence: {meta.lastSeq}</p>
          <p>Checkpoint count: {meta.session?.checkpointCount ?? 0}</p>
          <p>Cleanup verdict: {meta.session?.cleanupVerdict ?? 'n/a'}</p>
          <p>Reattachable: {meta.session?.reattachable ? 'yes' : 'no'}</p>
          {meta.gapState ? (
            <p className="warning-banner">
              Gap detected from {meta.gapState.fromSeq} to {meta.gapState.toSeq}.
            </p>
          ) : null}
          {guidance ? <p className="warning-banner">{guidance}</p> : null}
          {commandMessage ? <p>{commandMessage}</p> : null}
        </article>

        <article className="detail-card">
          <h3>Intervention controls</h3>
          <div className="decision-actions">
            <button
              onClick={() =>
                void stopSession(sessionId, 'graceful').then((ack) =>
                  setCommandMessage(formatAckMessage('Graceful stop', ack)),
                )
              }
              type="button"
            >
              Stop gracefully
            </button>
            <button
              onClick={() =>
                void stopSession(sessionId, 'force').then((ack) =>
                  setCommandMessage(formatAckMessage('Forced stop', ack)),
                )
              }
              type="button"
            >
              Force stop
            </button>
            <button
              disabled={!interactive}
              onClick={() =>
                void sendSessionInput(sessionId, inputValue).then((ack) =>
                  setCommandMessage(formatAckMessage('Input sent', ack)),
                )
              }
              type="button"
            >
              Send input
            </button>
            <button
              disabled={!interactive}
              onClick={() =>
                void resizeSession(sessionId, 132, 48).then((ack) =>
                  setCommandMessage(formatAckMessage('Resize applied', ack)),
                )
              }
              type="button"
            >
              Resize PTY
            </button>
            <button
              onClick={() =>
                void loadSessionHistory({
                  sessionId,
                  fromSeq: meta.gapState?.fromSeq ?? 1,
                  toSeq: meta.gapState?.toSeq ?? Math.max(meta.lastSeq, 1),
                  maxBytes: 8192,
                }).then((envelope) => {
                  setHistory(envelope.data);
                  setCommandMessage(
                    `Replay ${envelope.data.availability} for ${envelope.data.sessionId}`,
                  );
                })
              }
              type="button"
            >
              Request replay
            </button>
          </div>

          <label>
            PTY input
            <textarea
              disabled={!interactive}
              onChange={(event) => setInputValue(event.currentTarget.value)}
              rows={4}
              value={inputValue}
            />
          </label>

          {!interactive ? (
            <p className="empty-state">
              Input and resize controls are disabled for detached, contract, or terminal sessions.
            </p>
          ) : null}
        </article>
      </section>

      <article className="detail-card" aria-label="Session output">
        <h3>{viewerMode === 'live' ? 'Live terminal output' : 'Historical replay'}</h3>
        {viewerMode === 'live' ? (
          <pre>{tail || 'No PTY bytes have arrived yet.'}</pre>
        ) : (
          <div className="stack-list">
            {history?.frames.length ? (
              history.frames.map((frame) => (
                <div key={`${frame.kind}-${frame.seq}`}>
                  <strong>
                    {frame.kind} #{frame.seq}
                  </strong>
                  <pre>{frame.text}</pre>
                </div>
              ))
            ) : (
              <p className="empty-state">
                History availability: {history?.availability ?? 'missing'}
              </p>
            )}
          </div>
        )}
      </article>

      <article className="detail-card" aria-label="Structured timeline">
        <h3>Structured timeline</h3>
        {timeline.length === 0 ? (
          <p className="empty-state">
            Structured events will appear here once the bridge projects them.
          </p>
        ) : (
          <ul className="stack-list">
            {timeline.map((frame, index) => (
              <li key={`${frame.kind}-${index}`}>{frameText(frame)}</li>
            ))}
          </ul>
        )}
      </article>
    </section>
  );
}
