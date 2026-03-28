import { type SessionFrame } from './session-registry';

type PtySegment = {
  seq: number;
  chunk: Uint8Array;
};

export interface SessionBufferSnapshot {
  mode: 'pty' | 'contract';
  ptySegments: PtySegment[];
  frames: SessionFrame[];
}

export interface SessionBufferApi {
  getSnapshot(): SessionBufferSnapshot;
  subscribe(onStoreChange: () => void): () => void;
  appendFrame(frame: SessionFrame): void;
  mergeReplay(frames: SessionFrame[]): void;
  markGap(fromSeq: number, toSeq: number): void;
  prune(options?: { maxPtyBytes?: number; maxFrames?: number }): void;
  getTailText(maxBytes?: number): string;
}

function hasSequence(frame: SessionFrame): frame is Exclude<SessionFrame, { kind: 'gap'; fromSeq: number; toSeq: number }> {
  return frame.kind !== 'gap';
}

function frameIdentity(frame: SessionFrame): string {
  return hasSequence(frame) ? `seq:${frame.seq}` : `gap:${frame.fromSeq}:${frame.toSeq}`;
}

function decodeSegments(segments: PtySegment[]): string {
  return new TextDecoder().decode(
    segments.reduce((accumulator, segment) => {
      const next = new Uint8Array(accumulator.length + segment.chunk.length);
      next.set(accumulator);
      next.set(segment.chunk, accumulator.length);
      return next;
    }, new Uint8Array()),
  );
}

export function createSessionBuffer(mode: 'pty' | 'contract'): SessionBufferApi {
  let snapshot: SessionBufferSnapshot = {
    mode,
    ptySegments: [],
    frames: [],
  };
  const subscribers = new Set<() => void>();

  const emit = (): void => {
    subscribers.forEach((subscriber) => subscriber());
  };

  const upsertFrame = (frame: SessionFrame): boolean => {
    if (mode === 'pty' && frame.kind === 'pty') {
      if (snapshot.ptySegments.some((segment) => segment.seq === frame.seq)) {
        return false;
      }
      snapshot = {
        ...snapshot,
        ptySegments: [...snapshot.ptySegments, { seq: frame.seq, chunk: frame.chunk }],
      };
      return true;
    }

    const identities = new Set(snapshot.frames.map(frameIdentity));
    if (identities.has(frameIdentity(frame))) {
      return false;
    }

    const frames = [...snapshot.frames, frame].sort((left, right) => {
      if (!hasSequence(left) && !hasSequence(right)) {
        return left.fromSeq - right.fromSeq;
      }
      if (!hasSequence(left)) {
        return -1;
      }
      if (!hasSequence(right)) {
        return 1;
      }
      return left.seq - right.seq;
    });

    snapshot = {
      ...snapshot,
      frames,
    };
    return true;
  };

  return {
    getSnapshot: () => snapshot,
    subscribe: (onStoreChange) => {
      subscribers.add(onStoreChange);
      return () => subscribers.delete(onStoreChange);
    },
    appendFrame: (frame) => {
      if (upsertFrame(frame)) {
        emit();
      }
    },
    mergeReplay: (frames) => {
      let changed = false;
      frames.forEach((frame) => {
        changed = upsertFrame(frame) || changed;
      });
      if (changed) {
        emit();
      }
    },
    markGap: (fromSeq, toSeq) => {
      if (upsertFrame({ kind: 'gap', fromSeq, toSeq })) {
        emit();
      }
    },
    prune: ({ maxPtyBytes = 4096, maxFrames = 64 } = {}) => {
      let nextSegments = [...snapshot.ptySegments];
      while (
        nextSegments.reduce((total, segment) => total + segment.chunk.length, 0) > maxPtyBytes &&
        nextSegments.length > 1
      ) {
        nextSegments = nextSegments.slice(1);
      }

      const gapFrames = snapshot.frames.filter((frame) => frame.kind === 'gap');
      const nonGapFrames = snapshot.frames.filter((frame) => frame.kind !== 'gap');
      const nextFrames = [...gapFrames, ...nonGapFrames.slice(-maxFrames)];

      snapshot = {
        ...snapshot,
        ptySegments: nextSegments,
        frames: nextFrames,
      };
      emit();
    },
    getTailText: (maxBytes = 512) => {
      const allText = decodeSegments(snapshot.ptySegments);
      return allText.slice(-maxBytes);
    },
  };
}
