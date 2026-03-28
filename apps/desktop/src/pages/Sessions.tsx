import { useEffect, useState } from 'react';
import type { ReactElement } from 'react';

import {
  loadSessionsPage,
  retrySessionJob,
  useSessionMeta,
  type KeysetPage,
  type MutationAck,
  type QueryEnvelope,
  type SessionListFilters,
  type SessionPageItem,
} from '../hooks/use-daemon';

type SessionsPageProps = {
  loadSessions?: (
    filters: SessionListFilters,
    limit?: number,
  ) => Promise<QueryEnvelope<KeysetPage<SessionPageItem>>>;
  retryJob?: (jobId: string) => Promise<MutationAck>;
};

const DEFAULT_LIMIT = 25;

function createFilters(
  activeOnly: boolean,
  executionMode: 'all' | 'pty' | 'contract',
): SessionListFilters {
  return {
    activeOnly,
    executionModes: executionMode === 'all' ? undefined : [executionMode],
  };
}

function phaseTone(phase: SessionPageItem['phase']): string {
  if (phase === 'active') {
    return 'Live';
  }
  if (phase === 'terminating') {
    return 'Stopping';
  }
  if (phase === 'archived') {
    return 'Archived';
  }
  return `${phase.slice(0, 1).toUpperCase()}${phase.slice(1)}`;
}

function SessionRow({ sessionId }: { sessionId: string }): ReactElement | null {
  const meta = useSessionMeta(sessionId);
  const session = meta.session;

  if (!session) {
    return null;
  }

  return (
    <article data-session-id={session.sessionId}>
      <h3>{session.skillPath}</h3>
      <p>{phaseTone(session.phase)}</p>
      <p>{session.executionMode}</p>
      <p>{session.errorType ?? 'No error'}</p>
      <p>{session.cleanupVerdict ?? 'No cleanup verdict'}</p>
      <p>Last seq {meta.lastSeq}</p>
      <a href={`/sessions/${session.sessionId}`}>Open session</a>
    </article>
  );
}

export function SessionsPage({
  loadSessions = loadSessionsPage,
  retryJob = retrySessionJob,
}: SessionsPageProps): ReactElement {
  const [activeOnly, setActiveOnly] = useState(false);
  const [executionMode, setExecutionMode] = useState<'all' | 'pty' | 'contract'>('all');
  const [page, setPage] = useState<KeysetPage<SessionPageItem> | null>(null);
  const [snapshotSeq, setSnapshotSeq] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryMessage, setRetryMessage] = useState<string | null>(null);

  const filters = createFilters(activeOnly, executionMode);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    loadSessions(filters, DEFAULT_LIMIT)
      .then((envelope) => {
        if (cancelled) {
          return;
        }
        setPage(envelope.data);
        setSnapshotSeq(envelope.snapshotSeq);
      })
      .catch((loadError) => {
        if (cancelled) {
          return;
        }
        setError(loadError instanceof Error ? loadError.message : 'Unable to load sessions');
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [activeOnly, executionMode, loadSessions]);

  async function retryFirstRetryable(): Promise<void> {
    const retryable = page?.items.find((session) => session.jobId);
    if (!retryable?.jobId) {
      return;
    }

    setRetryMessage(`Retrying ${retryable.sessionId}...`);
    const ack = await retryJob(retryable.jobId);
    let envelope = await loadSessions(filters, DEFAULT_LIMIT);
    let attempts = 0;
    while (envelope.snapshotSeq < ack.visibleAfterSeq && attempts < 5) {
      attempts += 1;
      envelope = await loadSessions(filters, DEFAULT_LIMIT);
    }
    setPage(envelope.data);
    setSnapshotSeq(envelope.snapshotSeq);
    setRetryMessage(
      envelope.snapshotSeq >= ack.visibleAfterSeq
        ? `Retry queued for ${retryable.sessionId}`
        : `Retry pending for ${retryable.sessionId}`,
    );
  }

  return (
    <section data-route="/sessions">
      <h2>Recent sessions</h2>
      <p>Snapshot seq {snapshotSeq ?? 'pending'}</p>
      <label>
        <input
          checked={activeOnly}
          onChange={(event) => setActiveOnly(event.currentTarget.checked)}
          type="checkbox"
        />
        Active only
      </label>
      <label>
        Mode
        <select
          value={executionMode}
          onChange={(event) =>
            setExecutionMode(event.currentTarget.value as 'all' | 'pty' | 'contract')
          }
        >
          <option value="all">All</option>
          <option value="pty">PTY</option>
          <option value="contract">Contract</option>
        </select>
      </label>
      <button type="button" onClick={() => void retryFirstRetryable()}>
        Retry first failed job
      </button>
      {retryMessage ? <p>{retryMessage}</p> : null}
      {loading ? <p>Loading sessions…</p> : null}
      {error ? <p>{error}</p> : null}
      <div>
        {page?.items.map((session) => (
          <SessionRow key={session.sessionId} sessionId={session.sessionId} />
        ))}
      </div>
      {page && page.items.length === 0 ? <p>No sessions match the current filters.</p> : null}
    </section>
  );
}

export default SessionsPage;
