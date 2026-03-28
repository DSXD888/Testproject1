import type { SessionFrame } from '../stores/session-registry';
import type { InvokePayload } from './transport';

type DevEnvelope<T> = {
  data: T;
  snapshotSeq: number;
  snapshotAt: string;
};

type DevKeysetPage<T> = {
  items: T[];
  nextCursor: string | null;
  hasMore: boolean;
};

const SNAPSHOT_AT = '2026-03-28T20:00:00Z';
const SNAPSHOT_SEQ = 42;

const processListItem = {
  processId: 'proc-1',
  canonicalProcessId: 'proc-1',
  title: 'Disposable glove framework',
  buyerName: 'North Borough Council',
  publishedAt: '2026-03-28T09:00:00Z',
  deadlineAt: '2026-04-01T10:00:00Z',
  pipelineState: 'review',
  pricedCoverageBps: 8_000,
  netQualifiedCoverageBps: 7_000,
  netQualifiedValueCoverageBps: 7_100,
  grossMarginBps: 2_500,
  netMarginBaseBps: 2_000,
  candidateCount: 1,
  quotedCandidateCount: 1,
  decision: {
    disposition: 'watch',
    isStale: false,
    decidedAt: '2026-03-28T09:30:00Z',
  },
} as const;

const processDetail = {
  processId: 'proc-1',
  canonicalProcessId: 'proc-1',
  notices: [
    {
      noticeId: 'notice-1',
      sourceId: 'fts',
      sourceNoticeId: 'source-1',
      title: 'Disposable glove framework',
      buyerName: 'North Borough Council',
      procurementStage: 'tender',
      accessRoute: 'https://example.test/notice/1',
      deadlineAt: '2026-04-01T10:00:00Z',
      publishedAt: '2026-03-28T09:00:00Z',
      estimatedValueGbpExVatPence: 250_000,
    },
  ],
  lots: [
    {
      lotId: 'lot-1',
      noticeId: 'notice-1',
      lotNumber: 1,
      title: 'Lot 1',
      description: 'Core PPE',
      estimatedValuePence: 250_000,
      deadlineAt: '2026-04-01T10:00:00Z',
      isBidableSeparately: true,
    },
  ],
  candidates: [
    {
      candidateId: 'cand-1',
      lotId: 'lot-1',
      name: 'Disposable gloves',
      candidateType: 'product',
      matchArchetype: 'standard_class_led',
      headNoun: 'gloves',
      quoteabilityScoreBps: 8_100,
      candidateValueShareBps: 9_000,
    },
  ],
  quotes: [
    {
      quoteId: 'quote-1',
      candidateId: 'cand-1',
      productName: 'Disposable gloves',
      sourceName: 'Supplier A',
      sourceUrl: 'https://example.test/quote/1',
      unitPriceGbpPence: 1_200,
      minimumOrderQuantity: '100',
      leadTimeDays: 5,
      priceSignalRole: 'primary',
      matchConfidenceBps: 7_800,
      quoteUsabilityConfidenceBps: 7_400,
    },
  ],
  landedCosts: [
    {
      estimateId: 'landed-1',
      candidateId: 'cand-1',
      supplierQuoteId: 'quote-1',
      netMarginLowBps: 1_800,
      netMarginBaseBps: 2_000,
      netMarginHighBps: 2_200,
      decisionGrade: 'screening',
      confidenceBps: 7_600,
    },
  ],
  manifest: {
    markdown: '# Summary\n\n## Risks\n\nText',
    basisHash: 'basis-1',
    generatedAt: '2026-03-28T09:00:00Z',
    sections: ['Summary', 'Risks'],
    artifactHash: 'artifact-hash-1',
    stageRunId: 'stage-run-1',
  },
  coverage: {
    pricedCoverageBps: 8_000,
    landedCostCoverageBps: 7_500,
    netQualifiedCoverageBps: 7_000,
    pricedValueCoverageBps: 8_200,
    landedCostValueCoverageBps: 7_600,
    netQualifiedValueCoverageBps: 7_100,
    unresolvedMandatoryLots: 0,
  },
  decision: processListItem.decision,
  errors: [
    {
      errorManifestId: 'error-1',
      processId: 'proc-1',
      pipelineStage: 'qualify',
      errorType: 'AGENT_CLEANUP_INCOMPLETE',
      errorMessage: 'cleanup issue',
      cleanupVerdict: 'clean',
      severity: 'critical',
      skillPath: 'skills/procurement-ingestion/fts/SKILL.md',
      createdAt: '2026-03-28T09:10:00Z',
      resolved: false,
      agentSessionId: 'session-1',
    },
  ],
} as const;

const errorPage = {
  items: [
    {
      errorManifestId: 'error-1',
      processId: 'proc-1',
      pipelineStage: 'qualify',
      errorType: 'AGENT_CLEANUP_INCOMPLETE',
      errorMessage: 'cleanup issue',
      cleanupVerdict: 'clean',
      severity: 'critical',
      skillPath: 'skills/procurement-ingestion/fts/SKILL.md',
      createdAt: '2026-03-28T09:10:00Z',
      resolved: false,
      agentSessionId: 'session-1',
    },
  ],
  nextCursor: null,
  hasMore: false,
} as const;

const daemonInfo = {
  instanceId: 'daemon-1',
  daemonVersion: '1.2.3',
  protocolVersion: 1,
  schemaEpoch: 1,
  socketPath: '/tmp/procintel/daemon.sock',
  descriptorPath: '/tmp/procintel/daemon.info.json',
  stateRoot: '/tmp/procintel/state',
  stateDbPath: '/tmp/procintel/state/state.db',
  artifactRoot: '/tmp/procintel/artifacts',
  walBytes: 512,
  startedAt: '2026-03-28T18:00:00Z',
  startupRecovery: {
    recoveredInterruptedSessions: 0,
    lastRecoveryAt: null,
  },
} as const;

const sessionFixtures = [
  {
    sessionId: 'session-live',
    daemonInstanceId: 'daemon-1',
    skillPath: 'skills/procurement-ingestion/fts/SKILL.md',
    skillVersion: '1.0.0',
    executionMode: 'pty',
    phase: 'active',
    startedAt: '2026-03-28T09:00:00Z',
    endedAt: null,
    durationMs: null,
    cleanupVerdict: null,
    errorType: null,
    exitCode: null,
    checkpointCount: 2,
    byteCount: 18,
    reattachable: true,
    jobId: 'job-live',
  },
  {
    sessionId: 'session-failed',
    daemonInstanceId: 'daemon-1',
    skillPath: 'skills/procurement-ingestion/contracts-finder/SKILL.md',
    skillVersion: '1.0.0',
    executionMode: 'contract',
    phase: 'archived',
    startedAt: '2026-03-28T08:00:00Z',
    endedAt: '2026-03-28T08:02:00Z',
    durationMs: 120_000,
    cleanupVerdict: 'forced',
    errorType: 'AGENT_AUTH_CHALLENGE',
    exitCode: 1,
    checkpointCount: 1,
    byteCount: 64,
    reattachable: false,
    jobId: 'job-failed',
    archiveSeq: 40,
  },
] as const;

const liveSessionFrames: SessionFrame[] = [
  {
    seq: 1,
    kind: 'event',
    event: {
      type: 'started',
      pid: 1234,
      pgid: 1234,
      sid: 1234,
      mode: 'pty',
    },
    ts: '2026-03-28T09:00:00Z',
  },
  {
    seq: 2,
    kind: 'pty',
    chunk: new TextEncoder().encode('Connecting to supplier portal...\n'),
    ts: '2026-03-28T09:00:01Z',
  },
  {
    seq: 3,
    kind: 'event',
    event: {
      type: 'authChallenge',
      kind: 'mfa',
      detail: 'Browser sign-in required before the PTY session can continue.',
      intervention: {
        type: 'reveal_profile_path',
        profilePath: '/tmp/procintel/browser-profile',
        reattachable: true,
      },
    },
    ts: '2026-03-28T09:00:02Z',
  },
  {
    kind: 'gap',
    fromSeq: 6,
    toSeq: 8,
  },
];

export function seedDevSessionFrames(sessionId: string): SessionFrame[] {
  if (sessionId === 'session-live') {
    return liveSessionFrames;
  }
  return [];
}

function sessionHistoryPayload(
  sessionId: string,
  fromSeq: number,
  toSeq: number,
) {
  if (sessionId === 'session-live') {
    return {
      sessionId,
      firstSeq: fromSeq,
      lastSeq: toSeq,
      availability: 'complete' as const,
      frames: [
        {
          seq: 6,
          kind: 'pty' as const,
          chunk: Array.from(new TextEncoder().encode('Recovered frame six\n')),
          ts: '2026-03-28T09:00:06Z',
        },
        {
          seq: 7,
          kind: 'pty' as const,
          chunk: Array.from(new TextEncoder().encode('Recovered frame seven\n')),
          ts: '2026-03-28T09:00:07Z',
        },
        {
          seq: 8,
          kind: 'pty' as const,
          chunk: Array.from(new TextEncoder().encode('Recovered frame eight\n')),
          ts: '2026-03-28T09:00:08Z',
        },
      ],
    };
  }

  return {
    sessionId,
    firstSeq: 1,
    lastSeq: 3,
    availability: 'complete' as const,
    frames: [
      {
        seq: 1,
        kind: 'stdout' as const,
        chunk: 'Contracts Finder session booted.',
        ts: '2026-03-28T08:00:00Z',
      },
      {
        seq: 2,
        kind: 'stderr' as const,
        chunk: 'MFA required in managed browser.',
        ts: '2026-03-28T08:00:10Z',
      },
      {
        seq: 3,
        kind: 'event' as const,
        event: {
          type: 'finalResultReady' as const,
          path: '/tmp/procintel/contracts-finder/result.json',
        },
        ts: '2026-03-28T08:02:00Z',
      },
    ],
  };
}

function envelope<T>(data: T): DevEnvelope<T> {
  return {
    data,
    snapshotSeq: SNAPSHOT_SEQ,
    snapshotAt: SNAPSHOT_AT,
  };
}

function processIdFromArgs(args: Record<string, unknown> | undefined): string {
  const processId =
    (typeof args?.processId === 'string' && args.processId) ||
    (typeof args?.process_id === 'string' && args.process_id) ||
    'proc-1';

  return processId === 'proc-alias' ? 'proc-1' : processId;
}

export async function handleDevBridgeInvoke(
  payload: InvokePayload,
): Promise<unknown> {
  const args = payload.args;

  if (payload.cmd === 'list_processes') {
    return envelope<DevKeysetPage<typeof processListItem>>({
      items: [processListItem],
      nextCursor: null,
      hasMore: false,
    });
  }

  if (payload.cmd === 'get_process_detail') {
    const processId = processIdFromArgs(args);
    if (processId !== 'proc-1') {
      return {
        code: 'not_found',
        message: `unknown process ${processId}`,
        retryable: false,
      };
    }
    return envelope(processDetail);
  }

  if (payload.cmd === 'list_errors') {
    return envelope(errorPage);
  }

  if (payload.cmd === 'get_daemon_info') {
    return daemonInfo;
  }

  if (payload.cmd === 'list_sessions') {
    return envelope({
      items: sessionFixtures,
      nextCursor: null,
      hasMore: false,
    });
  }

  if (payload.cmd === 'get_session_history') {
    const sessionId =
      (typeof args?.sessionId === 'string' && args.sessionId) || 'session-failed';
    const fromSeq = typeof args?.fromSeq === 'number' ? args.fromSeq : 1;
    const toSeq = typeof args?.toSeq === 'number' ? args.toSeq : fromSeq;
    return envelope(sessionHistoryPayload(sessionId, fromSeq, toSeq));
  }

  if (
    payload.cmd === 'stop_session' ||
    payload.cmd === 'send_session_input' ||
    payload.cmd === 'resize_session'
  ) {
    return {
      accepted: true,
      visibleAfterSeq: SNAPSHOT_SEQ,
      sessionId:
        (typeof args?.sessionId === 'string' && args.sessionId) || 'session-live',
    };
  }

  return {
    code: 'dev_bridge_unconfigured',
    message: `No dev bridge handler for ${payload.cmd}`,
    retryable: false,
  };
}
