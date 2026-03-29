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
  phase: SessionPageItem['phase'] | 'all',
  search: string,
): SessionListFilters {
  return {
    activeOnly,
    executionModes: executionMode === 'all' ? undefined : [executionMode],
    phases: phase === 'all' ? undefined : [phase],
    search: search.trim() ? search.trim() : undefined,
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
      <p>Started {session.startedAt ?? 'Unknown start'}</p>
      <p>Ended {session.endedAt ?? 'In progress'}</p>
      <p>Duration {session.durationMs === null ? 'Live' : `${session.durationMs} ms`}</p>
      <p>{session.errorType ?? 'No error'}</p>
      <p>{session.cleanupVerdict ?? 'No cleanup verdict'}</p>
      <p>Exit code {session.exitCode ?? 'n/a'}</p>
      <p>Checkpoints {session.checkpointCount}</p>
      <p>Bytes {session.byteCount}</p>
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
  const [phase, setPhase] = useState<SessionPageItem['phase'] | 'all'>('all');
  const [search, setSearch] = useState('');
  const [page, setPage] = useState<KeysetPage<SessionPageItem> | null>(null);
  const [snapshotSeq, setSnapshotSeq] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryMessage, setRetryMessage] = useState<string | null>(null);

  const filters = createFilters(activeOnly, executionMode, phase, search);

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
  }, [activeOnly, executionMode, loadSessions, phase, search]);

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
      <label>
        Phase
        <select
          value={phase}
          onChange={(event) =>
            setPhase(
              event.currentTarget.value as SessionPageItem['phase'] | 'all',
            )
          }
        >
          <option value="all">All</option>
          <option value="created">Created</option>
          <option value="spawned">Spawned</option>
          <option value="active">Active</option>
          <option value="terminating">Terminating</option>
          <option value="exited">Exited</option>
          <option value="reaped">Reaped</option>
          <option value="archived">Archived</option>
        </select>
      </label>
      <label>
        Search
        <input
          value={search}
          onChange={(event) => setSearch(event.currentTarget.value)}
          placeholder="Session ID or skill path"
          type="search"
        />
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
