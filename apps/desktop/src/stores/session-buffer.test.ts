import { describe, expect, it } from 'vitest';

import { createSessionBuffer } from './session-buffer';
import { type SessionFrame } from './session-registry';

describe('session buffer', () => {
  it('TS-BUF-001 stores PTY bytes and derives tail text without mutating authoritative storage', () => {
    const buffer = createSessionBuffer('pty');
    buffer.appendFrame({
      seq: 1,
      kind: 'pty',
      chunk: new TextEncoder().encode('first\n'),
      ts: '2026-03-28T20:00:00Z',
    });
    buffer.appendFrame({
      seq: 2,
      kind: 'pty',
      chunk: new TextEncoder().encode('second\n'),
      ts: '2026-03-28T20:00:01Z',
    });

    const before = buffer.getSnapshot().ptySegments.length;
    const tail = buffer.getTailText();
    const after = buffer.getSnapshot().ptySegments.length;

    expect(tail).toContain('second');
    expect(before).toBe(after);
  });

  it('TS-BUF-002 preserves contract frame kinds and gap markers through replay merge', () => {
    const buffer = createSessionBuffer('contract');
    buffer.appendFrame({
      seq: 4,
      kind: 'stdout',
      chunk: 'alpha',
      ts: '2026-03-28T20:00:00Z',
    });
    buffer.markGap(5, 6);
    buffer.mergeReplay([
      {
        seq: 5,
        kind: 'event',
        event: { type: 'progress', completed: 1 },
        ts: '2026-03-28T20:00:01Z',
      },
    ]);

    expect(buffer.getSnapshot().frames.map((frame) => frame.kind)).toEqual([
      'gap',
      'stdout',
      'event',
    ]);
  });

  it('TS-BUF-003 replay merge inserts by original seq and deduplicates existing frames', () => {
    const buffer = createSessionBuffer('contract');
    const replayFrames: SessionFrame[] = [
      {
        seq: 3,
        kind: 'stdout',
        chunk: 'third',
        ts: '2026-03-28T20:00:03Z',
      },
      {
        seq: 2,
        kind: 'stdout',
        chunk: 'second',
        ts: '2026-03-28T20:00:02Z',
      },
      {
        seq: 2,
        kind: 'stdout',
        chunk: 'duplicate',
        ts: '2026-03-28T20:00:02Z',
      },
    ];

    buffer.mergeReplay(replayFrames);
    expect(
      buffer
        .getSnapshot()
        .frames.filter((frame) => frame.kind === 'stdout')
        .map((frame) => ('seq' in frame ? frame.seq : -1)),
    ).toEqual([2, 3]);
  });

  it('TS-BUF-004 pruning keeps unresolved gap markers while trimming tail context', () => {
    const buffer = createSessionBuffer('contract');
    buffer.markGap(2, 4);
    buffer.appendFrame({
      seq: 1,
      kind: 'stdout',
      chunk: 'one',
      ts: '2026-03-28T20:00:01Z',
    });
    buffer.appendFrame({
      seq: 5,
      kind: 'stdout',
      chunk: 'five',
      ts: '2026-03-28T20:00:05Z',
    });
    buffer.appendFrame({
      seq: 6,
      kind: 'stdout',
      chunk: 'six',
      ts: '2026-03-28T20:00:06Z',
    });

    buffer.prune({ maxFrames: 1 });

    expect(buffer.getSnapshot().frames[0]).toEqual({ kind: 'gap', fromSeq: 2, toSeq: 4 });
    expect(buffer.getSnapshot().frames.at(-1)).toMatchObject({ seq: 6, kind: 'stdout' });
  });
});
