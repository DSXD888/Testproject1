import type { IncomingMessage } from 'node:http';

import type { WebSocket } from 'ws';

import type { SessionFrame } from '../stores/session-registry';
import type { InvokePayload } from './transport';

type DevBridgeInvokeContext = {
  payload: InvokePayload;
  request: IncomingMessage;
  url: URL;
};

type DevBridgeWebSocketContext = {
  request: IncomingMessage;
  socket: WebSocket;
  url: URL;
};

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

type ErrorFixtureItem = {
  errorManifestId: string;
  processId: string | null;
  pipelineStage: string | null;
  errorType: string;
  errorMessage: string;
  cleanupVerdict: 'clean' | 'forced' | 'leaked' | null;
  severity: 'critical' | 'warning' | 'info';
  createdAt: string;
  resolved: boolean;
  agentSessionId: string | null;
};

type DecisionDisposition = 'approve' | 'reject' | 'watch' | null;

type BootstrapMode =
  | 'full_control'
  | 'control_limited'
  | 'browse_only'
  | 'incompatible'
  | 'recovery_required';

const SNAPSHOT_AT = '2026-03-28T20:00:00Z';
const SNAPSHOT_SEQ = 42;
const SESSION_STREAM_INTERVAL_MS = 20;
let restartScenarioCommitted = false;

function makeProcessSummary(
  processId: string,
  overrides: Partial<Record<string, unknown>> = {},
) {
  return {
    processId,
    canonicalProcessId: processId,
    title: `Process ${processId}`,
    buyerName: `Buyer ${processId}`,
    publishedAt: '2026-03-28T08:00:00Z',
    deadlineAt: '2026-04-30T12:00:00Z',
    estimatedValuePence: 1_250_000,
    lotCount: 2,
    accessRoute: 'public_web',
    legalCategory: 'goods',
    pipelineState: 'review',
    pricedCoverageBps: 7_000,
    netQualifiedCoverageBps: 5_000,
    netQualifiedValueCoverageBps: 4_500,
    grossMarginBps: 2_400,
    netMarginBaseBps: 1_800,
    candidateCount: 2,
    quotedCandidateCount: 1,
    decision: {
      disposition: 'watch',
      isStale: false,
      decidedAt: '2026-03-28T09:30:00Z',
    },
    ...overrides,
  };
}

type ProcessDetailFixture = {
  processId: string;
  canonicalProcessId: string;
  pipelineState: string;
  notices: Array<{
    noticeId: string;
    sourceId: string;
    sourceNoticeId: string;
    title: string;
    buyerName: string | null;
    procurementStage: string;
    summary: string | null;
    deadlineAt: string | null;
    publishedAt: string;
  }>;
  lots: Array<{
    lotId: string;
    noticeId: string;
    title: string | null;
    description: string | null;
    estimatedValuePence: number | null;
    deadlineAt: string | null;
    isBidableSeparately: boolean;
  }>;
  candidates: Array<{
    candidateId: string;
    lotId: string;
    name: string;
    candidateType: string;
    specificationText: string | null;
    quoteabilityScoreBps: number | null;
  }>;
  quotes: Array<{
    quoteId: string;
    productCandidateId: string | null;
    productName: string;
    sourceName: string;
    sourceUrl: string | null;
    unitPriceGbpPence: number;
    currency: string;
  }>;
  landedCosts: Array<{
    estimateId: string;
    productCandidateId: string;
    supplierQuoteId: string;
    economicLandedCostGbpPence: number;
    netMarginBaseBps: number;
    decisionGrade: string;
    confidenceBps: number;
  }>;
  manifest: {
    markdown: string;
    basisHash: string;
    generatedAt: string;
    sections: string[];
    artifactHash: string;
    stageRunId: string;
  } | null;
  reviewMetadata: {
    basisHash: string;
    manifestHash: string;
    stageRunId: string;
  };
  manifestError: { code: string; message: string } | null;
  coverage: {
    pricedCoverageBps: number;
    landedCostCoverageBps: number;
    netQualifiedCoverageBps: number;
    pricedValueCoverageBps: number;
    landedCostValueCoverageBps: number;
    netQualifiedValueCoverageBps: number;
    unresolvedMandatoryLots: number;
  };
  decision: {
    disposition: DecisionDisposition;
    isStale: boolean;
    decidedAt: string | null;
  };
  errors: Array<{
    errorManifestId: string;
    processId: string | null;
    pipelineStage: string | null;
    errorType: string;
    errorMessage: string;
    cleanupVerdict: 'clean' | 'forced' | 'leaked' | null;
    severity: 'critical' | 'warning' | 'info';
    createdAt: string;
    resolved: boolean;
    agentSessionId: string | null;
  }>;
};

function makeProcessDetail(processId: string): ProcessDetailFixture {
  return {
    processId,
    canonicalProcessId: processId,
    pipelineState: 'qualified',
  notices: [
    {
      noticeId: 'notice-1',
      sourceId: 'fts',
      sourceNoticeId: 'source-1',
      title: 'Steel widgets',
      buyerName: 'Buyer A',
      procurementStage: 'tender',
      summary: 'Procurement summary',
      deadlineAt: '2026-04-30T12:00:00Z',
      publishedAt: '2026-03-28T08:00:00Z',
    },
  ],
  lots: [
    {
      lotId: 'lot-1',
      noticeId: 'notice-1',
      title: 'Lot 1',
      description: 'Lot description',
      estimatedValuePence: 1_250_000,
      deadlineAt: '2026-04-30T12:00:00Z',
      isBidableSeparately: true,
    },
  ],
  candidates: [
    {
      candidateId: 'candidate-1',
      lotId: 'lot-1',
      name: 'Widget',
      candidateType: 'good',
      specificationText: 'Galvanised steel',
      quoteabilityScoreBps: 8_800,
    },
  ],
  quotes: [
    {
      quoteId: 'quote-1',
      productCandidateId: 'candidate-1',
      productName: 'Widget',
      sourceName: 'Supplier A',
      sourceUrl: 'https://example.test/quote',
      unitPriceGbpPence: 1_200,
      currency: 'GBP',
    },
  ],
  landedCosts: [
    {
      estimateId: 'landed-1',
      productCandidateId: 'candidate-1',
      supplierQuoteId: 'quote-1',
      economicLandedCostGbpPence: 1_720,
      netMarginBaseBps: 3_150,
      decisionGrade: 'screening',
      confidenceBps: 8_500,
    },
  ],
  manifest: {
    markdown: '## Summary\nStrong fit\n\n## Risks\nMonitor freight.',
    basisHash: 'basis-1',
    generatedAt: '2026-03-28T11:00:00Z',
    sections: ['Summary', 'Risks'],
    artifactHash: 'artifact-1',
    stageRunId: 'stage-run-1',
  },
  reviewMetadata: {
    basisHash: 'basis-1',
    manifestHash: 'artifact-1',
    stageRunId: 'stage-run-1',
  },
  manifestError: null,
  coverage: {
    pricedCoverageBps: 7_000,
    landedCostCoverageBps: 6_500,
    netQualifiedCoverageBps: 6_200,
    pricedValueCoverageBps: 7_100,
    landedCostValueCoverageBps: 6_100,
    netQualifiedValueCoverageBps: 4_200,
    unresolvedMandatoryLots: 1,
  },
  decision: {
    disposition: 'watch',
    isStale: true,
    decidedAt: '2026-03-28T09:30:00Z',
  },
  errors: [
    {
      errorManifestId: 'error-1',
      processId,
      pipelineStage: 'manifest',
      errorType: 'AGENT_CLEANUP_INCOMPLETE',
      errorMessage: 'Cleanup incomplete',
      cleanupVerdict: 'leaked',
      severity: 'critical',
      createdAt: '2026-03-28T11:05:00Z',
      resolved: false,
      agentSessionId: null,
    },
  ],
  };
}

const errorFixtures: Record<string, ErrorFixtureItem[]> = {
  default: [
    {
      errorManifestId: 'err-live-1',
      processId: 'proc-001',
      pipelineStage: 'Enrich',
      errorType: 'AGENT_DAEMON_CRASH_DURING_SESSION',
      errorMessage: 'Daemon crash interrupted the supplier enrichment session.',
      cleanupVerdict: 'leaked',
      severity: 'critical',
      createdAt: '2026-03-28T09:15:00Z',
      agentSessionId: 'session-live-1',
      resolved: false,
    },
    {
      errorManifestId: 'err-live-2',
      processId: 'proc-002',
      pipelineStage: 'Qualify',
      errorType: 'SOURCE_CHANGED',
      errorMessage: 'Notice page changed between qualification retries.',
      cleanupVerdict: null,
      severity: 'warning',
      createdAt: '2026-03-28T08:55:00Z',
      agentSessionId: 'session-live-2',
      resolved: false,
    },
  ],
  empty: [],
  hostile: [
    {
      errorManifestId: 'err-hostile-1',
      processId: 'proc-003',
      pipelineStage: 'Extract',
      errorType: 'AGENT_SKILL_FAILED',
      errorMessage: '<svg onload=payload-run>alert(1)</svg>',
      cleanupVerdict: 'forced',
      severity: 'warning',
      createdAt: '2026-03-28T07:00:00Z',
      agentSessionId: 'session-hostile-1',
      resolved: false,
    },
  ],
  severityMatrix: [
    {
      errorManifestId: 'err-severity-1',
      processId: 'proc-100',
      pipelineStage: 'Enrich',
      errorType: 'AGENT_DAEMON_CRASH_DURING_SESSION',
      errorMessage: 'Session daemon crashed before artifact sync.',
      cleanupVerdict: 'leaked',
      severity: 'critical',
      createdAt: '2026-03-28T06:45:00Z',
      agentSessionId: 'session-critical-1',
      resolved: false,
    },
    {
      errorManifestId: 'err-severity-2',
      processId: 'proc-101',
      pipelineStage: 'Qualify',
      errorType: 'SOURCE_CHANGED',
      errorMessage: 'Notice source changed during qualification.',
      cleanupVerdict: 'clean',
      severity: 'warning',
      createdAt: '2026-03-28T06:50:00Z',
      agentSessionId: 'session-warning-1',
      resolved: false,
    },
    {
      errorManifestId: 'err-severity-3',
      processId: 'proc-102',
      pipelineStage: 'Extract',
      errorType: 'PIPELINE_QUALITY_GATE_FAILED',
      errorMessage: 'Quality gate rejected the extracted packet.',
      cleanupVerdict: 'clean',
      severity: 'warning',
      createdAt: '2026-03-28T06:55:00Z',
      agentSessionId: 'session-warning-2',
      resolved: false,
    },
  ],
  resolveFence: [
    {
      errorManifestId: 'err-live-1',
      processId: 'proc-001',
      pipelineStage: 'Enrich',
      errorType: 'AGENT_DAEMON_CRASH_DURING_SESSION',
      errorMessage: 'Daemon crash interrupted the supplier enrichment session.',
      cleanupVerdict: 'leaked',
      severity: 'critical',
      createdAt: '2026-03-28T09:15:00Z',
      agentSessionId: 'session-live-1',
      resolved: false,
    },
  ],
  rerunFence: [
    {
      errorManifestId: 'err-live-1',
      processId: 'proc-001',
      pipelineStage: 'Enrich',
      errorType: 'AGENT_DAEMON_CRASH_DURING_SESSION',
      errorMessage: 'Daemon crash interrupted the supplier enrichment session.',
      cleanupVerdict: 'leaked',
      severity: 'critical',
      createdAt: '2026-03-28T09:15:00Z',
      agentSessionId: 'session-live-1',
      resolved: false,
    },
  ],
};

const errorFixtureState = new Map<
  string,
  { resolveReadsAfterAck: number; resolveRequested: boolean }
>();
const bridgeScenarioState = new Map<
  string,
  { rerunCalls: number; decisionCalls: number; detailReads: number; acknowledged: boolean }
>();

type DecisionFixtureState = {
  snapshotSeq: number;
  basisHash: string;
  decision: {
    disposition: DecisionDisposition;
    isStale: boolean;
    decidedAt: string | null;
  };
  manifestMode: 'normal' | 'metadata-only';
  reviewMetadata?: {
    basisHash: string;
    manifestHash: string;
    stageRunId: string;
  };
  delayedTransition?: {
    remainingReads: number;
    nextSnapshotSeq: number;
    nextDecision: DecisionFixtureState['decision'];
    nextBasisHash?: string;
  };
};

const decisionFixtureState = new Map<string, DecisionFixtureState>();

function cloneDecisionState(state: DecisionFixtureState): DecisionFixtureState {
  return {
    snapshotSeq: state.snapshotSeq,
    basisHash: state.basisHash,
    decision: { ...state.decision },
    manifestMode: state.manifestMode,
    reviewMetadata: state.reviewMetadata ? { ...state.reviewMetadata } : undefined,
    delayedTransition: state.delayedTransition
      ? { ...state.delayedTransition, nextDecision: { ...state.delayedTransition.nextDecision } }
      : undefined,
  };
}

function baseDecisionState(overrides: Partial<DecisionFixtureState>): DecisionFixtureState {
  return {
    snapshotSeq: 40,
    basisHash: 'basis-a',
    decision: {
      disposition: null,
      isStale: false,
      decidedAt: null,
    },
    manifestMode: 'normal',
    ...overrides,
  };
}

function decisionFixtureKey(request: IncomingMessage): string | null {
  return requestSearchParams(request).get('decisionFixture');
}

function bridgeFixtureKey(request: IncomingMessage): string | null {
  return requestSearchParams(request).get('bridgeFixture');
}

function securityFixtureKey(request: IncomingMessage): string | null {
  return requestSearchParams(request).get('securityFixture');
}

function currentBridgeScenarioState(request: IncomingMessage) {
  const key = bridgeFixtureKey(request);
  if (!key) {
    return null;
  }

  const existing = bridgeScenarioState.get(key);
  if (existing) {
    return existing;
  }

  const created = {
    rerunCalls: 0,
    decisionCalls: 0,
    detailReads: 0,
    acknowledged: false,
  };
  bridgeScenarioState.set(key, created);
  return created;
}

function currentDecisionFixtureState(request: IncomingMessage): DecisionFixtureState | null {
  const key = decisionFixtureKey(request);
  if (!key) {
    return null;
  }

  const existing = decisionFixtureState.get(key);
  if (existing) {
    return existing;
  }

  const created = (() => {
    switch (key) {
      case 'metadata-only':
        return baseDecisionState({
          snapshotSeq: 44,
          manifestMode: 'metadata-only',
          reviewMetadata: {
            basisHash: 'basis-metadata-only',
            manifestHash: 'artifact-process-metadata-only',
            stageRunId: 'stage-process-metadata-only',
          },
        });
      case 'watch-after-fence':
        return baseDecisionState({ snapshotSeq: 55 });
      case 'supersede':
        return baseDecisionState({ snapshotSeq: 70 });
      case 'reaffirm-watch':
        return baseDecisionState({
          snapshotSeq: 80,
          decision: {
            disposition: 'watch',
            isStale: false,
            decidedAt: '2026-03-28T20:55:00Z',
          },
        });
      case 'stale':
        return baseDecisionState({
          snapshotSeq: 91,
          basisHash: 'basis-b',
          decision: {
            disposition: 'watch',
            isStale: true,
            decidedAt: '2026-03-28T20:00:00Z',
          },
        });
      case 'fresh':
        return baseDecisionState({
          snapshotSeq: 92,
          decision: {
            disposition: 'watch',
            isStale: false,
            decidedAt: '2026-03-28T20:00:00Z',
          },
        });
      case 'mutation-failure':
        return baseDecisionState({ snapshotSeq: 100 });
      case 'rollback-failure':
        return baseDecisionState({
          snapshotSeq: 130,
          decision: {
            disposition: 'watch',
            isStale: false,
            decidedAt: '2026-03-28T21:15:00Z',
          },
        });
      case 'delayed-fence':
      case 'delayed-fence-commit':
      case 'delayed-fence-decisions':
        return baseDecisionState({
          snapshotSeq: 110,
          decision: {
            disposition: 'watch',
            isStale: false,
            decidedAt: '2026-03-28T20:00:00Z',
          },
        });
      case 'approve-after-fence':
      default:
        return baseDecisionState({ snapshotSeq: 40 });
    }
  })();

  decisionFixtureState.set(key, created);
  return created;
}

function decisionDetailEnvelope(
  processId: string,
  state: DecisionFixtureState,
) {
  const data = makeProcessDetail(processId);
  const manifest = data.manifest;
  if (!manifest) {
    throw new Error('decision detail fixture requires a manifest baseline');
  }
  data.manifest = {
    markdown: manifest.markdown,
    basisHash: state.basisHash,
    generatedAt: manifest.generatedAt,
    sections: manifest.sections,
    artifactHash: `artifact-${processId}`,
    stageRunId: `stage-${processId}`,
  };
  data.reviewMetadata = {
    basisHash: state.basisHash,
    manifestHash: `artifact-${processId}`,
    stageRunId: `stage-${processId}`,
  };
  data.decision = { ...state.decision };

  if (state.manifestMode === 'metadata-only' && state.reviewMetadata) {
    data.manifest = null;
    data.reviewMetadata = state.reviewMetadata;
    data.manifestError = {
      code: 'artifact_unavailable',
      message: 'manifest markdown unavailable',
    };
  }

  return {
    snapshotSeq: state.snapshotSeq,
    snapshotAt: '2026-03-28T21:00:00Z',
    data,
  };
}

function securityProcessDetail(request: IncomingMessage) {
  const fixtureKey = securityFixtureKey(request);
  if (!fixtureKey) {
    return null;
  }

  const detail = makeProcessDetail('process-sec');
  const manifest = detail.manifest;
  if (!manifest) {
    throw new Error('security detail fixture requires a manifest baseline');
  }
  detail.canonicalProcessId = 'process-sec';

  switch (fixtureKey) {
    case 'manifest-html':
      detail.manifest = {
        generatedAt: manifest.generatedAt,
        sections: manifest.sections,
        markdown: '## Summary\n<img src=x onerror=payload-run>\n<svg onload=payload-run></svg>',
        basisHash: 'basis-sec-1',
        artifactHash: 'artifact-sec-1',
        stageRunId: 'stage-sec-1',
      };
      return envelope(detail);
    case 'dangerous-links':
      detail.manifest = {
        generatedAt: manifest.generatedAt,
        sections: manifest.sections,
        markdown:
          '## Summary\n[safe](https://example.test)\n[bad](javascript:alert(1))\n[worse](data:text/html,<script>alert(1)</script>)',
        basisHash: 'basis-sec-2',
        artifactHash: 'artifact-sec-2',
        stageRunId: 'stage-sec-2',
      };
      return envelope(detail);
    case 'hostile-strings':
      detail.notices = [
        {
          noticeId: 'notice-1',
          sourceId: 'fts',
          sourceNoticeId: 'source-1',
          title: '<marquee>notice</marquee>',
          buyerName: '<b>buyer</b>',
          publishedAt: '2026-03-28T20:00:00Z',
          deadlineAt: null,
          procurementStage: 'tender',
          summary: '<script>summary</script>',
        },
      ];
      detail.candidates = [
        {
          candidateId: 'candidate-1',
          lotId: 'lot-1',
          name: '<script>candidate</script>',
          candidateType: 'good',
          specificationText: '<blink>specification</blink>',
          quoteabilityScoreBps: 5_000,
        },
      ];
      return envelope(detail);
    case 'large-payload':
      detail.manifest = {
        generatedAt: manifest.generatedAt,
        sections: manifest.sections,
        markdown: `## Summary\n${'0123456789abcdef'.repeat(700_000)}`,
        basisHash: 'basis-large',
        artifactHash: 'artifact-large',
        stageRunId: 'stage-large',
      };
      return envelope(detail);
    case 'unicode-spoof':
      detail.canonicalProcessId = 'safe\u202Ecod.exe';
      return envelope(detail);
    default:
      return envelope(detail);
  }
}

function securityErrorEnvelope(request: IncomingMessage) {
  const fixtureKey = securityFixtureKey(request);
  if (!fixtureKey) {
    return null;
  }

  switch (fixtureKey) {
    case 'dangerous-links':
      return errorEnvelope('[danger](javascript:alert(1)) and [safe](https://example.test)');
    case 'error-html':
      return errorEnvelope('<img src=x onerror=payload-run>');
    case 'hostile-strings':
      return errorEnvelope('<b>error</b>', '<i>TYPE</i>');
    case 'unicode-spoof':
      return errorEnvelope('warn\u200Fhidden');
    default:
      return null;
  }
}

function errorEnvelope(errorMessage: string, errorType = 'AGENT_SKILL_FAILED') {
  return envelope({
    items: [
      {
        errorManifestId: 'err-sec-1',
        processId: 'process-sec',
        pipelineStage: 'extract',
        errorType,
        errorMessage,
        cleanupVerdict: 'forced' as const,
        severity: 'warning' as const,
        createdAt: '2026-03-28T22:08:00Z',
        resolved: false,
        agentSessionId: null,
      },
    ],
    nextCursor: null,
    hasMore: false,
  });
}

function securitySessionListEnvelope(request: IncomingMessage) {
  const fixtureKey = securityFixtureKey(request);
  if (!fixtureKey) {
    return null;
  }

  switch (fixtureKey) {
    case 'osc8':
    case 'osc52':
    case 'hostile-strings':
    case 'large-payload':
    case 'unicode-spoof':
      return envelope({
        items: [
          {
            ...sessionFixtures[1],
            sessionId: 'session-sec',
            executionMode: 'contract',
            phase: 'exited',
            reattachable: false,
          },
        ],
        nextCursor: null,
        hasMore: false,
      });
    default:
      return null;
  }
}

function securitySessionHistoryEnvelope(request: IncomingMessage) {
  const fixtureKey = securityFixtureKey(request);
  if (!fixtureKey) {
    return null;
  }

  switch (fixtureKey) {
    case 'osc8':
      return envelope(
        sessionHistoryPayload(
          'session-sec',
          1,
          1,
          `start \u001b]8;;https://evil.test/hidden\u0007trusted label\u001b]8;;\u0007 end`,
          'stdout',
        ),
      );
    case 'osc52':
      return envelope(
        sessionHistoryPayload(
          'session-sec',
          1,
          1,
          `clipboard \u001b]52;c;YmFkLXNlY3JldA==\u0007 should not leak`,
          'stdout',
        ),
      );
    case 'hostile-strings':
      return envelope(sessionHistoryPayload('session-sec', 1, 1, '<blink>terminal</blink>', 'stdout'));
    case 'large-payload':
      return {
        snapshotSeq: 73,
        snapshotAt: '2026-03-28T22:10:01Z',
        data: {
          sessionId: 'session-sec',
          firstSeq: 1,
          lastSeq: 200,
          availability: 'complete',
          frames: Array.from({ length: 200 }, (_, index) => ({
            kind: 'stdout' as const,
            seq: index + 1,
            chunk: `line ${index + 1}`,
          })),
        },
      };
    case 'unicode-spoof':
      return envelope(sessionHistoryPayload('session-sec', 1, 1, 'session\u2066spoofed', 'stdout'));
    default:
      return null;
  }
}

function advanceDecisionFixtureRead(
  request: IncomingMessage,
  processId: string,
) {
  const state = currentDecisionFixtureState(request);
  if (!state) {
    return null;
  }

  if (state.delayedTransition) {
    if (state.delayedTransition.remainingReads <= 0) {
      state.snapshotSeq = state.delayedTransition.nextSnapshotSeq;
      state.decision = { ...state.delayedTransition.nextDecision };
      if (state.delayedTransition.nextBasisHash) {
        state.basisHash = state.delayedTransition.nextBasisHash;
      }
      state.delayedTransition = undefined;
    } else {
      state.delayedTransition.remainingReads -= 1;
    }
  }

  return decisionDetailEnvelope(processId, cloneDecisionState(state));
}

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

function envelope<T>(data: T): DevEnvelope<T> {
  return {
    data,
    snapshotSeq: SNAPSHOT_SEQ,
    snapshotAt: SNAPSHOT_AT,
  };
}

function requestSearchParams(request: IncomingMessage): URLSearchParams {
  const explicitSearch = request.headers['x-dev-page-search'];
  if (typeof explicitSearch === 'string' && explicitSearch.length > 0) {
    return new URLSearchParams(explicitSearch);
  }

  const referer = request.headers.referer;
  if (typeof referer !== 'string') {
    return new URLSearchParams();
  }

  try {
    return new URL(referer).searchParams;
  } catch {
    return new URLSearchParams();
  }
}

function requestedMode(params: URLSearchParams): BootstrapMode {
  const requestedMode = params.get('mode');

  switch (requestedMode) {
    case 'full-control':
      return 'full_control';
    case 'browse-only':
      return 'browse_only';
    case 'control-limited':
      return 'control_limited';
    case 'incompatible':
      return 'incompatible';
    case 'recovery':
      return 'recovery_required';
    default:
      return 'control_limited';
  }
}

function bootstrapSnapshotForParams(params: URLSearchParams) {
  const mode = requestedMode(params);
  const restartScenario = params.get('daemonFixture') === 'restart';
  const restartGeneration = restartScenario && restartScenarioCommitted ? 2 : 1;

  switch (mode) {
    case 'browse_only':
      return {
        mode,
        controlPlane: {
          daemonInstanceId: '',
          daemonVersion: '',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'ready' as const,
          daemon: 'unavailable' as const,
          control: 'read_only' as const,
          load: 'normal' as const,
          reasons: ['Daemon unavailable. Local snapshot is read-only.'],
        },
        descriptor: null,
        launchedDaemon: false,
      };
    case 'incompatible':
      return {
        mode,
        controlPlane: {
          daemonInstanceId: '',
          daemonVersion: '',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'schema_mismatch' as const,
          daemon: 'incompatible' as const,
          control: 'read_only' as const,
          load: 'normal' as const,
          reasons: ['Desktop and daemon are incompatible.'],
        },
        descriptor: null,
        launchedDaemon: false,
      };
    case 'recovery_required':
      return {
        mode,
        controlPlane: {
          daemonInstanceId: '',
          daemonVersion: '',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'repairing' as const,
          daemon: 'unavailable' as const,
          control: 'read_only' as const,
          load: 'normal' as const,
          reasons: ['Storage recovery is required.'],
        },
        descriptor: null,
        launchedDaemon: false,
      };
    case 'control_limited':
    case 'full_control':
    default:
      return {
        mode,
        controlPlane: {
          daemonInstanceId: restartScenario ? `daemon-${restartGeneration}` : 'daemon-dev',
          daemonVersion: '0.1.0',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'ready' as const,
          daemon:
            mode === 'full_control' || restartScenario
              ? ('ready' as const)
              : ('degraded' as const),
          control:
            mode === 'full_control' || restartScenario
              ? ('full' as const)
              : ('limited' as const),
          load:
            mode === 'full_control' || restartScenario
              ? ('normal' as const)
              : ('busy' as const),
          reasons:
            mode === 'full_control'
              ? ['Browser dev bridge is active.']
              : restartScenario
            ? ['Daemon restarted cleanly.']
            : ['Browser dev bridge is active. Some actions are unavailable.'],
        },
        descriptor: {
          instanceId: restartScenario ? `daemon-${restartGeneration}` : 'daemon-dev',
          pid: 1,
          socketPath: '/tmp/procintel/runtime/daemon.sock',
          protocolVersion: 1,
          schemaEpoch: 1,
          stateRoot: '/tmp/procintel/state',
          startedAt: '2026-03-28T12:00:00Z',
          ready: true,
        },
        launchedDaemon: false,
      };
  }
}

function bootstrapSnapshotForRequest(request: IncomingMessage) {
  return bootstrapSnapshotForParams(requestSearchParams(request));
}

function daemonInfoForRequest(request: IncomingMessage) {
  const params = requestSearchParams(request);
  const bootstrap = bootstrapSnapshotForRequest(request);
  const restartScenario = params.get('daemonFixture') === 'restart';
  const restartGeneration = restartScenario && restartScenarioCommitted ? 2 : 1;
  const descriptorRequested = params.get('daemonFixture') === 'descriptor';
  const liveAvailable =
    !descriptorRequested &&
    bootstrap.mode !== 'browse_only' &&
    bootstrap.mode !== 'incompatible' &&
    bootstrap.mode !== 'recovery_required';

  return {
    instanceId: liveAvailable
      ? bootstrap.controlPlane.daemonInstanceId ||
        (restartScenario ? `daemon-${restartGeneration}` : 'daemon-dev')
      : '',
    daemonVersion: liveAvailable ? bootstrap.controlPlane.daemonVersion || '0.1.0' : '',
    protocolVersion: bootstrap.controlPlane.protocolVersion,
    schemaEpoch: bootstrap.controlPlane.schemaEpoch,
    socketPath: '/tmp/procintel/runtime/daemon.sock',
    descriptorPath: '/tmp/procintel/runtime/daemon.info.json',
    stateRoot: '/tmp/procintel/state',
    stateDbPath: '/tmp/procintel/state/state.db',
    artifactRoot: '/tmp/procintel/artifacts',
    walBytes: liveAvailable ? 16_384 : null,
    startedAt: '2026-03-28T08:00:00Z',
    startupRecovery: {
      recoveredInterruptedSessions: liveAvailable ? 1 : 0,
      lastRecoveryAt: liveAvailable ? '2026-03-28T07:58:00Z' : null,
    },
    liveAvailable,
    descriptorBacked: !liveAvailable,
    storage: bootstrap.controlPlane.storage,
    daemon: liveAvailable ? bootstrap.controlPlane.daemon : 'unavailable',
    control: bootstrap.controlPlane.control,
  };
}

function errorsForRequest(request: IncomingMessage): ErrorFixtureItem[] {
  const fixtureKey = requestSearchParams(request).get('errorsFixture') ?? 'default';
  if (fixtureKey === 'resolveFence') {
    const state =
      errorFixtureState.get(fixtureKey) ??
      { resolveReadsAfterAck: 0, resolveRequested: false };
    errorFixtureState.set(fixtureKey, state);
    if (!state.resolveRequested) {
      return errorFixtures.resolveFence;
    }
    state.resolveReadsAfterAck += 1;
    return state.resolveReadsAfterAck >= 2 ? [] : errorFixtures.resolveFence;
  }
  return errorFixtures[fixtureKey] ?? errorFixtures.default;
}

function sessionListItems(payload: InvokePayload) {
  const rawFilters =
    payload.args &&
    typeof payload.args.filters === 'object' &&
    payload.args.filters !== null
      ? (payload.args.filters as Record<string, unknown>)
      : null;
  const activeOnly = rawFilters?.activeOnly === true;
  const executionModes = Array.isArray(rawFilters?.executionModes)
    ? rawFilters.executionModes.filter(
        (value): value is 'pty' | 'contract' =>
          value === 'pty' || value === 'contract',
      )
    : [];
  const phases = Array.isArray(rawFilters?.phases)
    ? rawFilters.phases.filter(
        (
          value,
        ): value is
          | 'created'
          | 'spawned'
          | 'active'
          | 'terminating'
          | 'exited'
          | 'reaped'
          | 'archived' =>
          typeof value === 'string',
      )
    : [];
  const search =
    typeof rawFilters?.search === 'string'
      ? rawFilters.search.trim().toLowerCase()
      : '';

  return sessionFixtures.filter((session) => {
    if (activeOnly && session.phase !== 'active') {
      return false;
    }
    if (executionModes.length > 0 && !executionModes.includes(session.executionMode)) {
      return false;
    }
    if (phases.length > 0 && !phases.includes(session.phase)) {
      return false;
    }
    if (
      search &&
      !session.sessionId.toLowerCase().includes(search) &&
      !session.skillPath.toLowerCase().includes(search) &&
      !(session.errorType ?? '').toLowerCase().includes(search)
    ) {
      return false;
    }
    return true;
  });
}

function filteredErrors(
  request: IncomingMessage,
  payload: InvokePayload,
): ErrorFixtureItem[] {
  const rawFilters =
    payload.args &&
    typeof payload.args.filters === 'object' &&
    payload.args.filters !== null
      ? (payload.args.filters as Record<string, unknown>)
      : null;
  const severities = Array.isArray(rawFilters?.severities)
    ? rawFilters.severities.filter(
        (value): value is ErrorFixtureItem['severity'] =>
          value === 'critical' || value === 'warning' || value === 'info',
      )
    : [];
  const pipelineStages = Array.isArray(rawFilters?.pipelineStages)
    ? rawFilters.pipelineStages.filter(
        (value): value is string => typeof value === 'string' && value.length > 0,
      )
    : [];
  const search =
    typeof rawFilters?.search === 'string'
      ? rawFilters.search.trim().toLowerCase()
      : '';

  return errorsForRequest(request).filter((item) => {
    if (severities.length > 0 && !severities.includes(item.severity)) {
      return false;
    }
    if (
      pipelineStages.length > 0 &&
      (!item.pipelineStage || !pipelineStages.includes(item.pipelineStage))
    ) {
      return false;
    }
    if (
      search &&
      !item.errorType.toLowerCase().includes(search) &&
      !item.errorMessage.toLowerCase().includes(search)
    ) {
      return false;
    }
    return true;
  });
}

function sessionHistoryPayload(
  sessionId: string,
  fromSeq: number,
  toSeq: number,
  contractText?: string,
  contractMode: 'pty' | 'stdout' = 'pty',
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
    lastSeq: contractText ? 1 : 3,
    availability: 'complete' as const,
    frames: contractText
      ? [
          {
            seq: 1,
            kind: contractMode,
            chunk:
              contractMode === 'pty'
                ? Array.from(new TextEncoder().encode(contractText))
                : contractText,
            ts: '2026-03-28T08:00:00Z',
          },
        ]
      : [
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

function processIdFromArgs(args: Record<string, unknown> | undefined): string {
  const processId =
    (typeof args?.processId === 'string' && args.processId) ||
    (typeof args?.process_id === 'string' && args.process_id) ||
    'proc-1';

  return processId === 'proc-alias' ? 'proc-1' : processId;
}

function serializeSessionFrame(frame: SessionFrame): Record<string, unknown> {
  if (frame.kind === 'pty') {
    return {
      ...frame,
      chunk: Array.from(frame.chunk),
    };
  }

  return frame;
}

function shouldEmitStreamFrame(frame: SessionFrame, fromSeq: number): boolean {
  if (frame.kind === 'gap') {
    return frame.toSeq > fromSeq;
  }

  return frame.seq > fromSeq;
}

export async function handleDevBridgeInvoke({
  payload,
  request,
}: DevBridgeInvokeContext): Promise<unknown> {
  const args = payload.args;
  const params = requestSearchParams(request);
  const restartScenario = params.get('daemonFixture') === 'restart';

  if (
    payload.cmd === 'get_bootstrap_snapshot' ||
    payload.cmd === 'refresh_bootstrap_snapshot'
  ) {
    if (payload.cmd === 'get_bootstrap_snapshot' && !restartScenario) {
      restartScenarioCommitted = false;
    }
    return bootstrapSnapshotForRequest(request);
  }

  if (payload.cmd === 'list_processes') {
    const rawFilters =
      args && typeof args.filters === 'object' && args.filters !== null
        ? (args.filters as Record<string, unknown>)
        : null;
    const sourceIds = Array.isArray(rawFilters?.sourceIds)
      ? rawFilters.sourceIds.filter((value): value is string => typeof value === 'string')
      : [];
    const pipelineStates = Array.isArray(rawFilters?.pipelineStates)
      ? rawFilters.pipelineStates.filter((value): value is string => typeof value === 'string')
      : [];
    const decisionDispositions = Array.isArray(rawFilters?.decisionDispositions)
      ? rawFilters.decisionDispositions.filter(
          (value): value is string => typeof value === 'string',
        )
      : [];
    const staleOnly = rawFilters?.staleOnly === true;
    const search =
      typeof rawFilters?.search === 'string' ? rawFilters.search.trim().toLowerCase() : '';

    const visible =
      (sourceIds.length === 0 || sourceIds.includes('fts')) &&
      (pipelineStates.length === 0 || pipelineStates.includes('review')) &&
      (decisionDispositions.length === 0 ||
        decisionDispositions.includes('watch')) &&
      (!staleOnly || true) &&
      (!search ||
        'Process process-1'.toLowerCase().includes(search) ||
        'Buyer process-1'.toLowerCase().includes(search));

    if (sourceIds.includes('empty')) {
      return envelope({
        items: [],
        nextCursor: null,
        hasMore: false,
      });
    }

    if (args?.cursor === 'cursor-page-2') {
      return envelope({
        items: Array.from({ length: 10 }, (_, index) =>
          makeProcessSummary(`process-${index + 26}`),
        ),
        nextCursor: null,
        hasMore: false,
      });
    }

    return envelope({
      items: visible
        ? Array.from({ length: 25 }, (_, index) =>
            makeProcessSummary(`process-${index + 1}`),
          )
        : [],
      nextCursor: 'cursor-page-2',
      hasMore: true,
    });
  }

  if (payload.cmd === 'get_process_detail') {
    const processId = processIdFromArgs(args);
    const securityDetail = securityProcessDetail(request);
    if (securityDetail) {
      return securityDetail;
    }
    const bridgeFixture = bridgeFixtureKey(request);
    const bridgeState = currentBridgeScenarioState(request);
    if (bridgeFixture === 'decision-ack-lost' && bridgeState) {
      bridgeState.detailReads += 1;
      return {
        snapshotSeq:
          bridgeState.acknowledged && bridgeState.detailReads >= 2 ? 81 : 80,
        snapshotAt: '2026-03-28T22:00:00Z',
        data: {
          ...makeProcessDetail(processId),
          decision:
            bridgeState.acknowledged && bridgeState.detailReads >= 2
              ? {
                  disposition: 'approve',
                  isStale: false,
                  decidedAt: '2026-03-28T21:59:00Z',
                }
              : {
                  disposition: null,
                  isStale: false,
                  decidedAt: null,
                },
          manifest: {
            ...makeProcessDetail(processId).manifest,
            basisHash: 'basis-bridge',
            artifactHash: 'artifact-bridge',
            stageRunId: 'stage-bridge',
          },
          reviewMetadata: {
            basisHash: 'basis-bridge',
            manifestHash: 'artifact-bridge',
            stageRunId: 'stage-bridge',
          },
        },
      };
    }
    const decisionFixture = advanceDecisionFixtureRead(request, processId);
    if (decisionFixture) {
      return decisionFixture;
    }
    if (processId === 'zero-lots') {
      const detail = makeProcessDetail(processId);
      detail.lots = [];
      return envelope(detail);
    }
    if (processId === 'no-candidates') {
      const detail = makeProcessDetail(processId);
      detail.candidates = [];
      detail.quotes = [];
      detail.landedCosts = [];
      detail.coverage = {
        ...detail.coverage,
        pricedCoverageBps: 0,
        landedCostCoverageBps: 0,
        netQualifiedCoverageBps: 0,
        pricedValueCoverageBps: 0,
        landedCostValueCoverageBps: 0,
        netQualifiedValueCoverageBps: 0,
      };
      return envelope(detail);
    }
    return envelope(makeProcessDetail(processId));
  }

  if (payload.cmd === 'capture_decision') {
    const bridgeFixture = bridgeFixtureKey(request);
    const bridgeState = currentBridgeScenarioState(request);
    if (bridgeFixture === 'decision-ack-lost' && bridgeState) {
      bridgeState.decisionCalls += 1;
      if (bridgeState.decisionCalls === 1) {
        return {
          code: 'transport_unavailable',
          message: 'ack lost after commit',
          retryable: true,
        };
      }
      bridgeState.acknowledged = true;
      return {
        accepted: true,
        visibleAfterSeq: 81,
        idempotencyKey:
          (typeof args?.idempotencyKey === 'string' && args.idempotencyKey) ||
          'dev-bridge-decision',
      };
    }
    const decisionFixture = currentDecisionFixtureState(request);
    if (decisionFixture) {
      const disposition =
        typeof args?.disposition === 'string'
          ? (args.disposition as Exclude<DecisionDisposition, null>)
          : 'watch';
      const idempotencyKey =
        (typeof args?.idempotencyKey === 'string' && args.idempotencyKey) ||
        'dev-bridge-decision';
      const processId =
        (typeof args?.processId === 'string' && args.processId) || 'process-1';
      if (decisionFixtureKey(request) === 'mutation-failure') {
        return {
          code: 'storage_unreadable',
          message: 'decision write failed',
          retryable: false,
        };
      }
      if (decisionFixtureKey(request) === 'rollback-failure') {
        return {
          code: 'storage_unreadable',
          message: 'rollback prevented decision persistence',
          retryable: false,
        };
      }

      const nextDecision = {
        disposition,
        isStale: false,
        decidedAt:
          disposition === 'watch' && decisionFixture.snapshotSeq === 80
            ? '2026-03-28T21:04:00Z'
            : disposition === 'watch'
              ? '2026-03-28T21:02:00Z'
              : disposition === 'reject'
                ? '2026-03-28T21:03:00Z'
                : '2026-03-28T21:00:00Z',
      };
      let visibleAfterSeq = decisionFixture.snapshotSeq;

      switch (decisionFixtureKey(request)) {
        case 'watch-after-fence':
          decisionFixture.snapshotSeq = 56;
          decisionFixture.decision = nextDecision;
          visibleAfterSeq = 56;
          break;
        case 'supersede':
          decisionFixture.snapshotSeq = disposition === 'approve' ? 71 : 72;
          decisionFixture.decision = nextDecision;
          visibleAfterSeq = decisionFixture.snapshotSeq;
          break;
        case 'reaffirm-watch':
          decisionFixture.snapshotSeq = 81;
          decisionFixture.decision = nextDecision;
          visibleAfterSeq = 81;
          break;
        case 'delayed-fence':
        case 'delayed-fence-commit':
        case 'delayed-fence-decisions':
          decisionFixture.delayedTransition = {
            remainingReads: 1,
            nextSnapshotSeq: 112,
            nextDecision,
          };
          visibleAfterSeq = 112;
          break;
        case 'metadata-only':
          decisionFixture.snapshotSeq = 45;
          visibleAfterSeq = 45;
          break;
        case 'approve-after-fence':
        default:
          decisionFixture.snapshotSeq = 41;
          decisionFixture.decision = nextDecision;
          visibleAfterSeq = 41;
          break;
      }

      return {
        accepted: true,
        visibleAfterSeq,
        idempotencyKey,
        processId,
      };
    }
    return {
      accepted: true,
      visibleAfterSeq: SNAPSHOT_SEQ,
      idempotencyKey:
        (typeof args?.idempotencyKey === 'string' && args.idempotencyKey) ||
        'dev-bridge-decision',
    };
  }

  if (payload.cmd === 'trigger_ingestion' || payload.cmd === 'trigger_enrichment') {
    return {
      accepted: true,
      visibleAfterSeq: SNAPSHOT_SEQ,
      jobId:
        payload.cmd === 'trigger_ingestion'
          ? 'job-dev-ingestion'
          : 'job-dev-enrichment',
    };
  }

  if (payload.cmd === 'list_errors') {
    const securityErrors = securityErrorEnvelope(request);
    if (securityErrors) {
      return securityErrors;
    }
    const bridgeFixture = bridgeFixtureKey(request);
    if (
      bridgeFixture === 'resolve-transport' ||
      bridgeFixture === 'rerun-conflict-then-busy' ||
      bridgeFixture === 'rerun-reservation-conflict'
    ) {
      return envelope({
        items: errorFixtures.default,
        nextCursor: null,
        hasMore: false,
      });
    }
    return envelope({
      items: filteredErrors(request, payload),
      nextCursor: null,
      hasMore: false,
    });
  }

  if (payload.cmd === 'resolve_error') {
    if (bridgeFixtureKey(request) === 'resolve-transport') {
      return {
        code: 'transport_unavailable',
        message: 'socket /tmp/procintel/daemon.sock unreachable',
        retryable: true,
      };
    }
    const fixtureKey = requestSearchParams(request).get('errorsFixture') ?? 'default';
    if (fixtureKey === 'resolveFence') {
      const state =
        errorFixtureState.get(fixtureKey) ??
        { resolveReadsAfterAck: 0, resolveRequested: false };
      state.resolveRequested = true;
      state.resolveReadsAfterAck = 0;
      errorFixtureState.set(fixtureKey, state);
      return {
        accepted: true,
        visibleAfterSeq: 201,
      };
    }
    return {
      accepted: true,
      visibleAfterSeq: SNAPSHOT_SEQ,
    };
  }

  if (payload.cmd === 'rerun_from_error') {
    const bridgeFixture = bridgeFixtureKey(request);
    const bridgeState = currentBridgeScenarioState(request);
    if (bridgeFixture === 'rerun-conflict-then-busy' && bridgeState) {
      bridgeState.rerunCalls += 1;
      return bridgeState.rerunCalls === 1
        ? {
            code: 'conflict',
            message: 'job is not rerunnable because another operator holds the reservation',
            retryable: false,
          }
        : {
            code: 'busy',
            message: 'retry once the daemon job queue clears',
            retryable: true,
          };
    }
    if (bridgeFixture === 'rerun-reservation-conflict') {
      return {
        code: 'conflict',
        message: 'downstream Agent Mail file_reservation_paths conflict blocked the rerun',
        retryable: false,
      };
    }
    if (requestSearchParams(request).get('errorsFixture') === 'rerunFence') {
      return {
        accepted: true,
        visibleAfterSeq: 301,
        jobId: 'rerun-job-1',
      };
    }
    return {
      accepted: true,
      visibleAfterSeq: SNAPSHOT_SEQ,
      jobId: 'rerun-job-1',
    };
  }

  if (payload.cmd === 'get_daemon_info') {
    return daemonInfoForRequest(request);
  }

  if (payload.cmd === 'restart_daemon') {
    if (restartScenario) {
      restartScenarioCommitted = true;
      return {
        accepted: true,
        visibleAfterSeq: 90,
      };
    }
    return {
      accepted: true,
      visibleAfterSeq: SNAPSHOT_SEQ,
    };
  }

  if (payload.cmd === 'open_logs') {
    return '/tmp/procintel/runtime';
  }

  if (payload.cmd === 'open_artifacts') {
    return '/tmp/procintel/artifacts';
  }

  if (payload.cmd === 'quit_ui') {
    return 'Quit UI requested.';
  }

  if (payload.cmd === 'stop_background_daemon_and_quit') {
    return 'Stop background daemon and quit requested.';
  }

  if (payload.cmd === 'list_sessions') {
    const securitySessions = securitySessionListEnvelope(request);
    if (securitySessions) {
      return securitySessions;
    }
    const bridgeFixture = bridgeFixtureKey(request);
    if (bridgeFixture === 'session-list') {
      return envelope({
        items: [
          {
            ...sessionFixtures[0],
            sessionId: 'session-live',
          },
        ],
        nextCursor: null,
        hasMore: false,
      });
    }
    return envelope({
      items: sessionListItems(payload),
      nextCursor: null,
      hasMore: false,
    });
  }

  if (payload.cmd === 'retry_job') {
    return {
      accepted: true,
      visibleAfterSeq: SNAPSHOT_SEQ,
      jobId:
        (typeof args?.jobId === 'string' && args.jobId) || 'job-live',
    };
  }

  if (payload.cmd === 'get_session_history') {
    const securityHistory = securitySessionHistoryEnvelope(request);
    if (securityHistory) {
      return securityHistory;
    }
    const bridgeFixture = bridgeFixtureKey(request);
    if (bridgeFixture === 'session-input-invalid') {
      return envelope(sessionHistoryPayload('session-live', 1, 1));
    }
    if (bridgeFixture === 'session-resize-terminal') {
      return envelope(sessionHistoryPayload('session-live', 1, 1));
    }
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
    if (payload.cmd === 'send_session_input' && bridgeFixtureKey(request) === 'session-input-invalid') {
      return {
        code: 'invalid_params',
        message: 'inputUtf8 must be a non-empty string',
        retryable: false,
      };
    }
    if (payload.cmd === 'resize_session' && bridgeFixtureKey(request) === 'session-resize-terminal') {
      return {
        code: 'already_terminal',
        message: 'session is already archived',
        retryable: false,
      };
    }
    return {
      accepted: true,
      visibleAfterSeq: SNAPSHOT_SEQ,
      sessionId:
        (typeof args?.sessionId === 'string' && args.sessionId) || 'session-live',
    };
  }

  return {
    code: 'unsupported',
    message: `No dev bridge handler for ${payload.cmd}`,
    retryable: false,
  };
}

export function handleDevBridgeWebSocket({
  request,
  socket,
  url,
}: DevBridgeWebSocketContext): void {
  const topic = url.searchParams.get('topic');

  if (topic === 'daemon.health') {
    const bootstrap = bootstrapSnapshotForParams(url.searchParams);
    globalThis.setTimeout(() => {
      if (socket.readyState !== 1) {
        return;
      }

      socket.send(
        JSON.stringify({
          daemon: bootstrap.controlPlane.daemon,
          control: bootstrap.controlPlane.control,
          load: bootstrap.controlPlane.load,
          reasons: bootstrap.controlPlane.reasons,
          daemonInstanceId: bootstrap.controlPlane.daemonInstanceId,
          daemonVersion: bootstrap.controlPlane.daemonVersion,
          protocolVersion: bootstrap.controlPlane.protocolVersion,
          schemaEpoch: bootstrap.controlPlane.schemaEpoch,
        }),
      );
    }, 0);
    return;
  }

  if (topic !== 'sessions') {
    socket.close();
    return;
  }

  const fromSeq = Number(url.searchParams.get('fromSeq') ?? '0');
  const timers = liveSessionFrames
    .filter((frame) => shouldEmitStreamFrame(frame, Number.isFinite(fromSeq) ? fromSeq : 0))
    .map((frame, index) =>
      globalThis.setTimeout(() => {
        if (socket.readyState !== 1) {
          return;
        }

        socket.send(
          JSON.stringify({
            sessionId: 'session-live',
            frame: serializeSessionFrame(frame),
          }),
        );
      }, index * SESSION_STREAM_INTERVAL_MS),
    );

  socket.once('close', () => {
    timers.forEach((timer) => globalThis.clearTimeout(timer));
  });
}
