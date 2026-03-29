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

const processListItem = {
  processId: 'proc-1',
  canonicalProcessId: 'proc-1',
  title: 'Disposable glove framework',
  buyerName: 'North Borough Council',
  publishedAt: '2026-03-28T09:00:00Z',
  deadlineAt: '2026-04-01T10:00:00Z',
  estimatedValuePence: 250_000,
  lotCount: 1,
  accessRoute: 'public_web',
  legalCategory: 'goods',
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
  pipelineState: 'qualified',
  notices: [
    {
      noticeId: 'notice-1',
      sourceId: 'fts',
      sourceNoticeId: 'source-1',
      title: 'Disposable glove framework',
      buyerName: 'North Borough Council',
      procurementStage: 'tender',
      summary: 'Framework renewal for disposable gloves.',
      deadlineAt: '2026-04-01T10:00:00Z',
      publishedAt: '2026-03-28T09:00:00Z',
    },
  ],
  lots: [
    {
      lotId: 'lot-1',
      noticeId: 'notice-1',
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
      specificationText: 'Core nitrile gloves for frontline staff.',
      quoteabilityScoreBps: 8_100,
      candidateValueShareBps: 9_000,
    },
  ],
  quotes: [
    {
      quoteId: 'quote-1',
      productCandidateId: 'cand-1',
      productName: 'Disposable gloves',
      sourceName: 'Supplier A',
      sourceUrl: 'https://example.test/quote/1',
      unitPriceGbpPence: 1_200,
      currency: 'GBP',
      priceSignalRole: 'primary',
      shipFromCountry: 'GB',
    },
  ],
  landedCosts: [
    {
      estimateId: 'landed-1',
      productCandidateId: 'cand-1',
      supplierQuoteId: 'quote-1',
      economicLandedCostGbpPence: 1_450,
      netMarginBaseBps: 2_000,
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
  reviewMetadata: {
    basisHash: 'basis-1',
    manifestHash: 'artifact-hash-1',
    stageRunId: 'stage-run-1',
  },
  manifestError: null,
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
      createdAt: '2026-03-28T09:10:00Z',
      resolved: false,
      agentSessionId: 'session-1',
    },
  ],
} as const;

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
};

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

function bootstrapSnapshotForRequest(request: IncomingMessage) {
  const params = requestSearchParams(request);
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
    default:
      return {
        mode: 'control_limited' as const,
        controlPlane: {
          daemonInstanceId: restartScenario ? `daemon-${restartGeneration}` : 'daemon-dev',
          daemonVersion: '0.1.0',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'ready' as const,
          daemon: restartScenario ? ('ready' as const) : ('degraded' as const),
          control: restartScenario ? ('full' as const) : ('limited' as const),
          load: restartScenario ? ('normal' as const) : ('busy' as const),
          reasons: restartScenario
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
      (pipelineStates.length === 0 || pipelineStates.includes(processListItem.pipelineState)) &&
      (decisionDispositions.length === 0 ||
        decisionDispositions.includes(processListItem.decision.disposition ?? '')) &&
      (!staleOnly || processListItem.decision.isStale) &&
      (!search ||
        processListItem.title.toLowerCase().includes(search) ||
        processListItem.buyerName.toLowerCase().includes(search));

    return envelope<DevKeysetPage<typeof processListItem>>({
      items: visible ? [processListItem] : [],
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

  if (payload.cmd === 'capture_decision') {
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
    return envelope({
      items: filteredErrors(request, payload),
      nextCursor: null,
      hasMore: false,
    });
  }

  if (payload.cmd === 'resolve_error') {
    return {
      accepted: true,
      visibleAfterSeq: SNAPSHOT_SEQ,
    };
  }

  if (payload.cmd === 'rerun_from_error') {
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
    code: 'unsupported',
    message: `No dev bridge handler for ${payload.cmd}`,
    retryable: false,
  };
}

export function handleDevBridgeWebSocket({
  socket,
  url,
}: DevBridgeWebSocketContext): void {
  if (url.searchParams.get('topic') !== 'sessions') {
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
