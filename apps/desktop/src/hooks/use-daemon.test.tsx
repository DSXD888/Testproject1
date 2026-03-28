import { renderToStaticMarkup } from 'react-dom/server';
import { beforeEach, describe, expect, it } from 'vitest';

import {
  applySessionFrame,
  hydrateSessions,
  resetSessionDaemonState,
  useSessionMeta,
  useSessionTail,
  useSessionTimeline,
  type SessionPageItem,
} from './use-daemon';

function makeSession(sessionId: string, executionMode: SessionPageItem['executionMode'] = 'pty'): SessionPageItem {
  return {
    sessionId,
    daemonInstanceId: 'daemon-a',
    skillPath: 'skills/procurement-ingestion/fts/SKILL.md',
    skillVersion: '1.0.0',
    executionMode,
    phase: 'active',
    startedAt: '2026-03-28T10:00:00Z',
    endedAt: null,
    durationMs: null,
    cleanupVerdict: null,
    errorType: null,
    exitCode: null,
    checkpointCount: 0,
    byteCount: 0,
    reattachable: true,
    jobId: 'job-1',
  };
}

function MetaProbe() {
  const meta = useSessionMeta('session-1');
  return <div>{JSON.stringify(meta)}</div>;
}

function TailProbe() {
  const tail = useSessionTail('session-1');
  return <div>{tail}</div>;
}

function TimelineProbe() {
  const timeline = useSessionTimeline('session-1');
  return <div>{timeline.length}</div>;
}

describe('use-daemon hooks', () => {
  beforeEach(() => {
    resetSessionDaemonState('daemon-a');
  });

  it('reads session metadata slices through useSyncExternalStore', () => {
    hydrateSessions([makeSession('session-1')]);
    applySessionFrame('session-1', {
      kind: 'gap',
      fromSeq: 10,
      toSeq: 12,
    });

    const markup = renderToStaticMarkup(<MetaProbe />);
    expect(markup).toContain('lastSeq');
    expect(markup).toContain('fromSeq');
  });

  it('reads the PTY tail from the session buffer without exposing raw arrays', () => {
    hydrateSessions([makeSession('session-1', 'pty')]);
    applySessionFrame('session-1', {
      seq: 1,
      kind: 'pty',
      chunk: new TextEncoder().encode('hello world'),
      ts: '2026-03-28T10:00:01Z',
    });

    const markup = renderToStaticMarkup(<TailProbe />);
    expect(markup).toContain('hello world');
  });

  it('reads timeline slices for contract sessions', () => {
    hydrateSessions([makeSession('session-1', 'contract')]);
    applySessionFrame('session-1', {
      seq: 1,
      kind: 'event',
      event: { type: 'warning', code: 'warn', detail: 'detail' },
      ts: '2026-03-28T10:00:01Z',
    });

    const markup = renderToStaticMarkup(<TimelineProbe />);
    expect(markup).toContain('1');
  });
});
