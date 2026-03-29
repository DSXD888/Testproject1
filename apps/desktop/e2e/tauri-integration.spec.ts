import { expect, test, type Page } from '@playwright/test';

import {
  bootstrapFixture,
  daemonInfoFixture,
  processListEnvelope,
  type NativeFixture,
} from './fixtures/native';
import { installNativeHarness } from './support/native-harness';

test('TAU-INT-001 Quit UI leaves the daemon-owned runtime surface intact', async ({
  page,
}) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('full_control'),
    daemonInfo: daemonInfoFixture(),
    processList: processListEnvelope(),
    extra: {
      quit_ui: 'Quit UI requested.',
    },
  });

  await page.goto('/daemon');
  await page.getByRole('button', { name: 'Quit UI' }).click();
  await expect(page.getByText('Quit UI requested.')).toBeVisible();
  await expect(
    page.evaluate(
      () =>
        (window as typeof window & {
          __TAURI_CALLS__?: Array<{ cmd: string }>;
        }).__TAURI_CALLS__?.some((call) => call.cmd === 'quit_ui') ?? false,
    ),
  ).resolves.toBe(true);
});

test('TAU-INT-002 Stop background daemon and quit is a distinct desktop action', async ({
  page,
}) => {
  await installNativeHarness(page, {
    bootstrap: bootstrapFixture('full_control'),
    daemonInfo: daemonInfoFixture('daemon-native-2'),
    processList: processListEnvelope(),
    extra: {
      stop_background_daemon_and_quit: 'Stop background daemon and quit requested.',
    },
  });

  await page.goto('/daemon');
  await page.getByRole('button', { name: 'Stop background daemon and quit' }).click();
  await expect(page.getByText('Stop background daemon and quit requested.')).toBeVisible();
  await expect(
    page.evaluate(
      () =>
        (window as typeof window & {
          __TAURI_CALLS__?: Array<{ cmd: string }>;
        }).__TAURI_CALLS__?.some(
          (call) => call.cmd === 'stop_background_daemon_and_quit',
        ) ?? false,
    ),
  ).resolves.toBe(true);
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
