import { describe, expect, it } from 'vitest';

import {
  createSessionRegistry,
  type SessionFrame,
  type SessionListItem,
} from './session-registry';

function createMemoryStorage() {
  const values = new Map<string, string>();
  return {
    getItem(key: string): string | null {
      return values.get(key) ?? null;
    },
    setItem(key: string, value: string): void {
      values.set(key, value);
    },
  };
}

function makeSession(sessionId: string, daemonInstanceId = 'daemon-a'): SessionListItem {
  return {
    sessionId,
    daemonInstanceId,
    skillPath: 'skills/example/SKILL.md',
    skillVersion: '1.0.0',
    executionMode: 'pty',
    phase: 'active',
    startedAt: '2026-03-28T20:00:00Z',
    endedAt: null,
    durationMs: 1000,
    cleanupVerdict: null,
    errorType: null,
    exitCode: null,
    checkpointCount: 0,
    byteCount: 0,
    reattachable: true,
  };
}

describe('session registry', () => {
  it('TS-REG-001 stores sessions by daemonInstanceId + sessionId without clobbering unrelated sessions', () => {
    const registry = createSessionRegistry();
    registry.applySessionMeta(makeSession('session-1', 'daemon-a'));
    registry.applySessionMeta(makeSession('session-1', 'daemon-b'));

    const snapshot = registry.getSnapshot();
    expect(Object.keys(snapshot.sessions)).toEqual(['daemon-a:session-1', 'daemon-b:session-1']);
  });

  it('TS-REG-002 advances lastSeq monotonically and ignores out-of-order frames', () => {
    const registry = createSessionRegistry('daemon-a');
    registry.applySessionMeta(makeSession('session-1'));
    registry.applyStreamFrame('session-1', {
      seq: 4,
      kind: 'stdout',
      chunk: 'hello',
      ts: '2026-03-28T20:00:01Z',
    });
    registry.applyStreamFrame('session-1', {
      seq: 3,
      kind: 'stdout',
      chunk: 'older',
      ts: '2026-03-28T20:00:00Z',
    });

    expect(registry.getSnapshot().lastSeqBySession['daemon-a:session-1']).toBe(4);
  });

  it('TS-REG-003 records exact gap ranges and clears only the matching replay window', () => {
    const registry = createSessionRegistry('daemon-a');
    registry.applySessionMeta(makeSession('session-1'));
    registry.applyStreamFrame('session-1', {
      kind: 'gap',
      fromSeq: 10,
      toSeq: 14,
    });
    registry.markReplayResolved('session-1', 10, 13);
    expect(registry.getSnapshot().gapStateBySession['daemon-a:session-1']).toEqual({
      fromSeq: 10,
      toSeq: 14,
    });
    registry.markReplayResolved('session-1', 10, 14);
    expect(registry.getSnapshot().gapStateBySession['daemon-a:session-1']).toBeNull();
  });

  it('TS-REG-004 tracks archiveSeq and fences historical handoff until snapshot visibility', () => {
    const registry = createSessionRegistry('daemon-a');
    registry.applySessionMeta(makeSession('session-1'));
    registry.applyStreamFrame('session-1', {
      seq: 8,
      kind: 'lifecycle',
      state: 'archived',
      archiveSeq: 21,
      ts: '2026-03-28T20:00:02Z',
    });

    expect(registry.canPromoteHistorical('session-1', 20)).toBe(false);
    expect(registry.canPromoteHistorical('session-1', 21)).toBe(true);
  });

  it('TS-REG-005 preserves anchors across same-daemon reset and clears them on daemon change', () => {
    const storage = createMemoryStorage();
    const registry = createSessionRegistry('daemon-a', storage);
    registry.applySessionMeta(makeSession('session-1'));
    registry.applyStreamFrame('session-1', {
      seq: 9,
      kind: 'event',
      event: { type: 'progress', completed: 1 },
      ts: '2026-03-28T20:00:03Z',
    } satisfies SessionFrame);

    registry.resetForDaemon('daemon-a');
    expect(registry.getSnapshot().lastSeqBySession['daemon-a:session-1']).toBe(9);

    registry.resetForDaemon('daemon-b');
    expect(registry.getSnapshot().lastSeqBySession['daemon-a:session-1']).toBeUndefined();

    const reopened = createSessionRegistry('daemon-a', storage);
    reopened.applySessionMeta(makeSession('session-1'));
    expect(reopened.getSnapshot().lastSeqBySession['daemon-a:session-1']).toBe(9);
  });
});
