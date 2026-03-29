import type { Page } from '@playwright/test';

import type { NativeFixture } from '../fixtures/native';

export async function installNativeHarness(
  page: Page,
  fixture: NativeFixture,
): Promise<void> {
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
    (window as typeof window & { __TAURI_CALLS__?: typeof calls }).__TAURI_CALLS__ =
      calls;

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
          return (
            state.processDetail ?? {
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
            }
          );
        case 'list_sessions':
          return (
            state.sessions ?? {
              snapshotSeq: 44,
              snapshotAt: '2026-03-28T22:30:00Z',
              data: { items: [], nextCursor: null, hasMore: false },
            }
          );
        case 'get_session_history':
          return (
            state.sessionHistory ?? {
              snapshotSeq: 44,
              snapshotAt: '2026-03-28T22:30:00Z',
              data: {
                sessionId: 'session-native',
                firstSeq: 1,
                lastSeq: 1,
                availability: 'complete',
                frames: [],
              },
            }
          );
        default:
          return (
            state.extra?.[cmd] ?? {
              code: 'unsupported',
              message: `Unsupported native harness command ${cmd}`,
              retryable: false,
            }
          );
      }
    };

    (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ =
      {
        invoke: async (cmd: string, args?: Record<string, unknown>) => {
          calls.push({ cmd, args });
          return read(cmd);
        },
        subscribe: () => () => undefined,
      };
  }, fixture);
}
