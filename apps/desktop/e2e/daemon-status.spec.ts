import { expect, test } from '@playwright/test';

test('PW-DGN-001 shows live daemon diagnostics', async ({ page }) => {
  await page.goto('/daemon?daemonFixture=live');

  await expect(page.getByRole('heading', { name: 'Daemon status' })).toBeVisible();
  await expect(page.getByText('Instance ID')).toBeVisible();
  await expect(page.getByText('daemon-dev')).toBeVisible();
  await expect(page.getByText('Socket path')).toBeVisible();
  await expect(page.getByText('/tmp/procintel/runtime/daemon.sock')).toBeVisible();
  await expect(page.getByText('DB path')).toBeVisible();
  await expect(page.getByText('/tmp/procintel/state/state.db')).toBeVisible();
  await expect(page.getByText('WAL bytes')).toBeVisible();
  await expect(page.locator('.detail-card')).toContainText(
    'Recovered 1 interrupted session(s); last recovery at 2026-03-28T07:58:00Z.',
  );
  await expect(page.getByRole('button', { name: 'Open logs' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Open artifacts' })).toBeVisible();
});

test('PW-DGN-003 restarts the daemon only when restart is allowed and shows the new instance id', async ({
  page,
}) => {
  let infoReads = 0;

  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as {
      cmd: string;
    };

    if (body.cmd === 'get_bootstrap_snapshot') {
      await route.fulfill({
        json: {
          mode: 'full_control',
          controlPlane: {
            daemonInstanceId: 'daemon-1',
            daemonVersion: '0.1.0',
            protocolVersion: 1,
            schemaEpoch: 1,
            storage: 'ready',
            daemon: 'ready',
            control: 'full',
            load: 'normal',
            reasons: ['Desktop bootstrapped against a healthy daemon.'],
          },
          descriptor: null,
          launchedDaemon: false,
        },
      });
      return;
    }

    if (body.cmd === 'refresh_bootstrap_snapshot') {
      await route.fulfill({
        json: {
          mode: 'full_control',
          controlPlane: {
            daemonInstanceId: 'daemon-2',
            daemonVersion: '0.1.0',
            protocolVersion: 1,
            schemaEpoch: 1,
            storage: 'ready',
            daemon: 'ready',
            control: 'full',
            load: 'normal',
            reasons: ['Daemon restarted cleanly.'],
          },
          descriptor: null,
          launchedDaemon: false,
        },
      });
      return;
    }

    if (body.cmd === 'get_daemon_info') {
      infoReads += 1;
      await route.fulfill({
        json: {
          instanceId: infoReads === 1 ? 'daemon-1' : 'daemon-2',
          daemonVersion: '0.1.0',
          protocolVersion: 1,
          schemaEpoch: 1,
          socketPath: '/tmp/procintel/runtime/daemon.sock',
          descriptorPath: '/tmp/procintel/runtime/daemon.info.json',
          stateRoot: '/tmp/procintel/state',
          stateDbPath: '/tmp/procintel/state/state.db',
          artifactRoot: '/tmp/procintel/artifacts',
          walBytes: 8192,
          startedAt: '2026-03-28T08:00:00Z',
          startupRecovery: {
            recoveredInterruptedSessions: 1,
            lastRecoveryAt: '2026-03-28T07:58:00Z',
          },
        },
      });
      return;
    }

    if (body.cmd === 'restart_daemon') {
      await route.fulfill({
        json: {
          accepted: true,
          visibleAfterSeq: 90,
        },
      });
      return;
    }

    if (body.cmd === 'open_logs' || body.cmd === 'open_artifacts') {
      await route.fulfill({
        json:
          body.cmd === 'open_logs'
            ? '/tmp/procintel/runtime'
            : '/tmp/procintel/artifacts',
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/daemon');
  await expect(page.getByRole('button', { name: 'Restart daemon' })).toBeVisible();
  await page.getByRole('button', { name: 'Restart daemon' }).click();
  await expect(page.getByText('Restart accepted at visibility fence 90.')).toBeVisible();
  await expect(page.getByText('daemon-2')).toBeVisible();
});

test('PW-DGN-002 and PW-DGN-004 keep descriptor-backed diagnostics readable while gating restart', async ({
  page,
}) => {
  await page.goto('/daemon?daemonFixture=descriptor&mode=browse-only');

  await expect(
    page.getByText('Live daemon unavailable. Showing descriptor-backed diagnostics only.'),
  ).toBeVisible();
  await expect(page.getByText('Unavailable').first()).toBeVisible();
  await expect(page.getByRole('button', { name: 'Restart daemon' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Open logs' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Open artifacts' })).toBeVisible();
  await expect(page.getByText('descriptor-backed', { exact: true }).first()).toBeVisible();

  await page.goto('/daemon?daemonFixture=live&mode=control-limited');
  await expect(page.getByText('Diagnostics remain visible while mutations are gated by limited.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Restart daemon' })).toBeVisible();
});
