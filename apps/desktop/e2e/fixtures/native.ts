export type NativeFixture = {
  bootstrap: Record<string, unknown>;
  daemonInfo: Record<string, unknown>;
  processList: Record<string, unknown>;
  processDetail?: Record<string, unknown>;
  sessions?: Record<string, unknown>;
  sessionHistory?: Record<string, unknown>;
  extra?: Record<string, unknown>;
};

export function bootstrapFixture(
  mode:
    | 'full_control'
    | 'control_limited'
    | 'browse_only'
    | 'incompatible'
    | 'recovery_required',
  overrides: Partial<Record<string, unknown>> = {},
) {
  const controlPlane =
    mode === 'browse_only'
      ? {
          daemonInstanceId: '',
          daemonVersion: '',
          protocolVersion: 1,
          schemaEpoch: 1,
          storage: 'ready',
          daemon: 'unavailable',
          control: 'read_only',
          load: 'normal',
          reasons: ['Daemon unavailable. Local snapshot is read-only.'],
        }
      : mode === 'incompatible'
        ? {
            daemonInstanceId: '',
            daemonVersion: '',
            protocolVersion: 1,
            schemaEpoch: 1,
            storage: 'schema_mismatch',
            daemon: 'incompatible',
            control: 'read_only',
            load: 'normal',
            reasons: ['Desktop and daemon are incompatible.'],
          }
        : mode === 'recovery_required'
          ? {
              daemonInstanceId: '',
              daemonVersion: '',
              protocolVersion: 1,
              schemaEpoch: 1,
              storage: 'repairing',
              daemon: 'unavailable',
              control: 'read_only',
              load: 'normal',
              reasons: ['Storage recovery is required.'],
            }
          : mode === 'control_limited'
            ? {
                daemonInstanceId: 'daemon-limited',
                daemonVersion: '0.1.0',
                protocolVersion: 1,
                schemaEpoch: 1,
                storage: 'ready',
                daemon: 'degraded',
                control: 'limited',
                load: 'busy',
                reasons: ['Daemon is degraded. Some actions are unavailable.'],
              }
            : {
                daemonInstanceId: 'daemon-native',
                daemonVersion: '0.1.0',
                protocolVersion: 1,
                schemaEpoch: 1,
                storage: 'ready',
                daemon: 'ready',
                control: 'full',
                load: 'normal',
                reasons: ['Desktop dev fixture is active.'],
              };

  return {
    mode,
    controlPlane,
    descriptor: {
      instanceId: controlPlane.daemonInstanceId || 'daemon-native',
      pid: 42,
      socketPath: '/tmp/procintel/runtime/daemon.sock',
      protocolVersion: 1,
      schemaEpoch: 1,
      stateRoot: '/tmp/procintel/state',
      startedAt: '2026-03-28T08:00:00Z',
      ready: mode === 'full_control' || mode === 'control_limited',
    },
    launchedDaemon: false,
    ...overrides,
  };
}

export function processListEnvelope() {
  return {
    snapshotSeq: 44,
    snapshotAt: '2026-03-28T22:30:00Z',
    data: {
      items: [
        {
          processId: 'proc-native',
          canonicalProcessId: 'proc-native',
          title: 'Native harness process',
          buyerName: 'Native Buyer',
          publishedAt: '2026-03-28T08:00:00Z',
          deadlineAt: '2026-04-30T12:00:00Z',
          pipelineState: 'review',
          pricedCoverageBps: 7000,
          netQualifiedCoverageBps: 6000,
          netQualifiedValueCoverageBps: 6100,
          grossMarginBps: 2200,
          netMarginBaseBps: 1800,
          candidateCount: 1,
          quotedCandidateCount: 1,
          decision: {
            disposition: 'watch',
            isStale: false,
            decidedAt: '2026-03-28T09:00:00Z',
          },
        },
      ],
      nextCursor: null,
      hasMore: false,
    },
  };
}

export function daemonInfoFixture(instanceId = 'daemon-native') {
  return {
    instanceId,
    daemonVersion: '0.1.0',
    protocolVersion: 1,
    schemaEpoch: 1,
    socketPath: '/tmp/procintel/runtime/daemon.sock',
    descriptorPath: '/tmp/procintel/runtime/daemon.info.json',
    stateRoot: '/tmp/procintel/state',
    stateDbPath: '/tmp/procintel/state/state.db',
    artifactRoot: '/tmp/procintel/artifacts',
    walBytes: 2048,
    startedAt: '2026-03-28T08:00:00Z',
    startupRecovery: {
      recoveredInterruptedSessions: 0,
      lastRecoveryAt: null,
    },
  };
}
