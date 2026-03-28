import { expect, test, type Page } from '@playwright/test';

type NativeFixture = {
  bootstrap: Record<string, unknown>;
  daemonInfo: Record<string, unknown>;
  processList: Record<string, unknown>;
  processDetail?: Record<string, unknown>;
  sessions?: Record<string, unknown>;
  sessionHistory?: Record<string, unknown>;
  extra?: Record<string, unknown>;
};

function bootstrapFixture(
  mode: 'full_control' | 'control_limited' | 'browse_only' | 'incompatible' | 'recovery_required',
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

function processListEnvelope() {
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

function daemonInfoFixture(instanceId = 'daemon-native') {
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

async function installNativeHarness(page: Page, fixture: NativeFixture): Promise<void> {
  await page.route('**/__tauri_invoke__', async (route) => {
    await route.fulfill({
      status: 500,
      json: {
        code: 'dev_bridge_should_not_run',
        message: 'The native Tauri harness should bypass the dev bridge.',
      },
    });
  });

  await page.addInitScript((input) => {
    const state = structuredClone(input) as NativeFixture & { calls?: unknown[] };
    const calls: Array<{ cmd: string; args?: Record<string, unknown> }> = [];
    (window as typeof window & {
      __TAURI_FIXTURE__?: NativeFixture;
      __TAURI_CALLS__?: typeof calls;
    }).__TAURI_FIXTURE__ = state;
    (window as typeof window & { __TAURI_CALLS__?: typeof calls }).__TAURI_CALLS__ = calls;

    const read = (cmd: string): unknown => {
      switch (cmd) {
        case 'get_bootstrap_snapshot':
        case 'refresh_bootstrap_snapshot':
          return state.bootstrap;
        case 'get_daemon_info':
          return state.daemonInfo;
        case 'list_processes':
          return state.processList;
        case 'get_process_detail':
          return state.processDetail ?? {
            snapshotSeq: 44,
            snapshotAt: '2026-03-28T22:30:00Z',
            data: {
              processId: 'proc-native',
              canonicalProcessId: 'proc-native',
              notices: [],
              lots: [],
              candidates: [],
              quotes: [],
              landedCosts: [],
              manifest: null,
              manifestError: null,
              coverage: {
                pricedCoverageBps: 7000,
                landedCostCoverageBps: 7000,
                netQualifiedCoverageBps: 7000,
                pricedValueCoverageBps: 7000,
                landedCostValueCoverageBps: 7000,
                netQualifiedValueCoverageBps: 7000,
                unresolvedMandatoryLots: 0,
              },
              decision: { disposition: null, isStale: false, decidedAt: null },
              errors: [],
            },
          };
        case 'list_sessions':
          return state.sessions ?? {
            snapshotSeq: 44,
            snapshotAt: '2026-03-28T22:30:00Z',
            data: { items: [], nextCursor: null, hasMore: false },
          };
        case 'get_session_history':
          return state.sessionHistory ?? {
            snapshotSeq: 44,
            snapshotAt: '2026-03-28T22:30:00Z',
            data: {
              sessionId: 'session-native',
              firstSeq: 1,
              lastSeq: 1,
              availability: 'complete',
              frames: [],
            },
          };
        default:
          return state.extra?.[cmd] ?? {
            code: 'unsupported',
            message: `Unsupported native harness command ${cmd}`,
            retryable: false,
          };
      }
    };

    (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {
      invoke: async (cmd: string, args?: Record<string, unknown>) => {
        calls.push({ cmd, args });
        return read(cmd);
      },
      subscribe: () => () => undefined,
    };
  }, fixture);
}

test('TAU-INT-001 native bootstrap uses the Tauri invoke path for the queue shell', async ({
  page,
}) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('full_control'),
    daemonInfo: daemonInfoFixture(),
    processList: processListEnvelope(),
  });

  await page.goto('/');
  await expect(page.getByText('Native harness process')).toBeVisible();
  await expect(page.locator('.desktop-shell__status-banner')).toContainText('full control');
  await expect(
    page.evaluate(() => (window as typeof window & { __TAURI_CALLS__?: unknown[] }).__TAURI_CALLS__?.length ?? 0),
  ).resolves.toBeGreaterThan(0);
});

test('TAU-INT-002 native diagnostics use invoke without the dev bridge', async ({ page }) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('full_control'),
    daemonInfo: daemonInfoFixture('daemon-native-2'),
    processList: processListEnvelope(),
  });

  await page.goto('/daemon');
  await expect(page.getByText('daemon-native-2')).toBeVisible();
  await expect(page.getByText('/tmp/procintel/runtime/daemon.sock')).toBeVisible();
});

test('TAU-INT-003 auto-launch bootstrap can still resolve to full control', async ({ page }) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('full_control', { launchedDaemon: true }),
    daemonInfo: daemonInfoFixture(),
    processList: processListEnvelope(),
  });

  await page.goto('/');
  await expect(page.locator('.desktop-shell__status-banner')).toContainText('full control');
});

test('TAU-INT-004 readable storage plus unavailable daemon maps to browse-only', async ({
  page,
}) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('browse_only'),
    daemonInfo: daemonInfoFixture(''),
    processList: processListEnvelope(),
  });

  await page.goto('/');
  await expect(page.locator('.desktop-shell__status-banner')).toContainText('browse only');
});

test('TAU-INT-005 incompatible native bootstrap renders the mismatch mode', async ({
  page,
}) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('incompatible'),
    daemonInfo: daemonInfoFixture(''),
    processList: processListEnvelope(),
  });

  await page.goto('/');
  await expect(page.locator('.desktop-shell__status-banner')).toContainText('incompatible');
});

test('TAU-INT-006 unreadable or repairing storage renders recovery-required mode', async ({
  page,
}) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('recovery_required'),
    daemonInfo: daemonInfoFixture(''),
    processList: processListEnvelope(),
  });

  await page.goto('/');
  await expect(page.locator('.desktop-shell__status-banner')).toContainText('recovery required');
});

test('TAU-INT-007 degraded daemon maps to control-limited mode', async ({ page }) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('control_limited'),
    daemonInfo: daemonInfoFixture('daemon-limited'),
    processList: processListEnvelope(),
  });

  await page.goto('/');
  await expect(page.locator('.desktop-shell__status-banner')).toContainText('control limited');
});

test('TAU-INT-008 the five-mode matrix stays total and non-overlapping', async ({ page }) => {
  const fixtures = [
    bootstrapFixture('full_control'),
    bootstrapFixture('control_limited'),
    bootstrapFixture('browse_only'),
    bootstrapFixture('incompatible'),
    bootstrapFixture('recovery_required'),
  ];

  const observedModes = new Set<string>();
  for (const bootstrap of fixtures) {
    await installNativeHarness(page, {
      bootstrap,
      daemonInfo: daemonInfoFixture(String(bootstrap.controlPlane['daemonInstanceId'] ?? '')),
      processList: processListEnvelope(),
    });
    await page.goto('/');
    observedModes.add(
      await page.locator('.desktop-shell__status-banner strong').textContent() ?? '',
    );
  }

  expect(observedModes).toEqual(
    new Set([
      'full control',
      'control limited',
      'browse only',
      'incompatible',
      'recovery required',
    ]),
  );
});

test('TAU-INT-009 same-daemon reconnect can retain the instance id', async ({ page }) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('full_control', {
      controlPlane: {
        ...bootstrapFixture('full_control').controlPlane,
        daemonInstanceId: 'daemon-same',
      },
    }),
    daemonInfo: daemonInfoFixture('daemon-same'),
    processList: processListEnvelope(),
  });

  await page.goto('/daemon');
  await expect(page.getByText('daemon-same')).toBeVisible();
  await page.reload();
  await expect(page.getByText('daemon-same')).toBeVisible();
});

test('TAU-INT-010 changed instance ids can invalidate live resume assumptions', async ({
  page,
}) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('browse_only', {
      controlPlane: {
        ...bootstrapFixture('browse_only').controlPlane,
        reasons: ['Daemon restart invalidated live PTY resume.'],
      },
    }),
    daemonInfo: daemonInfoFixture('daemon-restarted'),
    processList: processListEnvelope(),
  });

  await page.goto('/');
  await expect(page.getByText('Daemon restart invalidated live PTY resume.')).toBeVisible();
});

test('TAU-INT-011 native and browser payload shapes stay app-level compatible', async ({
  page,
}) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('full_control'),
    daemonInfo: daemonInfoFixture(),
    processList: processListEnvelope(),
  });

  await page.goto('/');
  await expect(page.getByTestId('contract-row')).toHaveCount(1);
  await expect(page.getByTestId('contract-row').first()).toContainText('Native harness process');
});

test('TAU-INT-012 the native harness avoids the dev bridge control path entirely', async ({
  page,
}) => {
  let devBridgeHits = 0;
  await page.route('**/__tauri_invoke__', async (route) => {
    devBridgeHits += 1;
    await route.fulfill({
      status: 500,
      json: {
        code: 'dev_bridge_should_not_run',
        message: 'The native Tauri harness should bypass the dev bridge.',
      },
    });
  });

  await page.addInitScript((fixture) => {
    (window as typeof window & {
      __TAURI_INTERNALS__?: unknown;
    }).__TAURI_INTERNALS__ = {
      invoke: async (cmd: string) => {
        if (cmd === 'get_bootstrap_snapshot' || cmd === 'refresh_bootstrap_snapshot') {
          return fixture.bootstrap;
        }
        if (cmd === 'list_processes') {
          return fixture.processList;
        }
        if (cmd === 'get_daemon_info') {
          return fixture.daemonInfo;
        }
        return { code: 'unsupported', message: `unsupported ${cmd}`, retryable: false };
      },
      subscribe: () => () => undefined,
    };
  }, {
    bootstrap: bootstrapFixture('full_control'),
    daemonInfo: daemonInfoFixture(),
    processList: processListEnvelope(),
  } satisfies NativeFixture);

  await page.goto('/');
  await page.goto('/daemon');
  expect(devBridgeHits).toBe(0);
});
