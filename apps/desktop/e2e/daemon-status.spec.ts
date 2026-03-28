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
});

test('PW-DGN-002 and PW-DGN-004 keep descriptor-backed diagnostics readable in browse-only mode', async ({
  page,
}) => {
  await page.goto('/daemon?daemonFixture=descriptor&mode=browse-only');

  await expect(
    page.getByText('Live daemon unavailable. Showing descriptor-backed diagnostics only.'),
  ).toBeVisible();
  await expect(page.getByText('Unavailable').first()).toBeVisible();
  await expect(page.getByRole('button', { name: 'Restart daemon' })).toBeDisabled();
  await expect(page.getByText('descriptor-backed', { exact: true }).first()).toBeVisible();

  await page.goto('/daemon?daemonFixture=live&mode=control-limited');
  await expect(page.getByText('Diagnostics remain visible while mutations are gated by limited.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Restart daemon' })).toBeDisabled();
});
