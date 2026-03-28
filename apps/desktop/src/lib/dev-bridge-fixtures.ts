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
      items: [
        {
          sessionId: 'session-1',
          daemonInstanceId: 'daemon-1',
          skillPath: 'skills/procurement-ingestion/fts/SKILL.md',
          skillVersion: '1.0.0',
          executionMode: 'pty',
          phase: 'archived',
          startedAt: '2026-03-28T09:00:00Z',
          endedAt: '2026-03-28T09:05:00Z',
          durationMs: 300_000,
          cleanupVerdict: 'clean',
          errorType: null,
          exitCode: 0,
          checkpointCount: 2,
          byteCount: 18,
          reattachable: false,
        },
      ],
      nextCursor: null,
      hasMore: false,
    });
  }

  if (payload.cmd === 'get_session_history') {
    return envelope({
      sessionId: 'session-1',
      firstSeq: 1,
      lastSeq: 1,
      availability: 'complete',
      frames: [
        {
          seq: 1,
          kind: 'pty',
          chunk: [104, 101, 108, 108, 111, 10],
          ts: '2026-03-28T09:00:01Z',
        },
      ],
    });
  }

  return {
    code: 'dev_bridge_unconfigured',
    message: `No dev bridge handler for ${payload.cmd}`,
    retryable: false,
  };
}
