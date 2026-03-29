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
  await page.goto('/daemon?daemonFixture=restart&mode=control-limited');
  await expect(page.getByRole('button', { name: 'Restart daemon' })).toBeVisible();
  await page.getByRole('button', { name: 'Restart daemon' }).click();
  await expect(
    page.getByText('Daemon restarted. Instance changed from daemon-1 to daemon-2.'),
  ).toBeVisible();
  await expect(page.getByText('daemon-2', { exact: true })).toBeVisible();
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
