import { expect, test } from '../../e2e-playwright-compat';

test.describe('sessions page', () => {
  test('PW-SES-001 renders active and recent sessions from the query envelope', async ({
    page,
  }) => {
    await page.goto('/sessions');
    await expect(page.getByRole('heading', { name: 'Recent sessions' })).toBeVisible();
    await expect(page.locator('[data-session-id="session-live"]')).toContainText(
      'skills/procurement-ingestion/fts/SKILL.md',
    );
    await expect(page.locator('[data-session-id="session-failed"]')).toContainText(
      'AGENT_AUTH_CHALLENGE',
    );
  });

  test('PW-SES-002 keeps failed-before-active sessions visible with canonical phase metadata', async ({
    page,
  }) => {
    await page.goto('/sessions');
    await expect(page.locator('[data-session-id="session-failed"]')).toContainText('Archived');
    await expect(page.locator('[data-session-id="session-failed"]')).toContainText('forced');
  });

  test('PW-SES-005 filters to active sessions only', async ({ page }) => {
    await page.goto('/sessions');
    await page.getByLabel('Active only').check();
    await expect(page.locator('[data-session-id="session-live"]')).toBeVisible();
    await expect(page.locator('[data-session-id="session-failed"]')).toHaveCount(0);
  });

  test('sessions page supports phase-filtered views without opening every transcript', async ({
    page,
  }) => {
    await page.goto('/sessions');
    await page.getByLabel('Phase').selectOption('archived');
    await expect(page.locator('[data-session-id="session-failed"]')).toBeVisible();
    await expect(page.locator('[data-session-id="session-live"]')).toHaveCount(0);
  });

  test('PW-SES-011 retry waits for the visibility fence before reporting success', async ({
    page,
  }) => {
    await page.goto('/sessions');
    await page.getByRole('button', { name: 'Retry first failed job' }).click();
    await expect(page.getByText('Retry queued for session-live')).toBeVisible();
  });

  test('PW-SES-003 renders live PTY monitoring with replay and auth guidance', async ({
    page,
  }) => {
    await page.goto('/sessions/session-live');
    await expect(page.getByText('Live terminal')).toBeVisible();
    await expect(page.getByText('Browser auth challenge')).toBeVisible();
    await expect(page.locator('p.page__status').filter({ hasText: 'Replay recovered 6:8.' })).toBeVisible();
    await expect(
      page.locator('pre.session-viewer__terminal').filter({ hasText: 'Recovered frame eight' }),
    ).toBeVisible();
  });

  test('PW-SES-004 renders contract sessions in transcript/history mode with input disabled', async ({
    page,
  }) => {
    await page.goto('/sessions/session-failed?initialMode=history');
    await expect(page.getByText('Transcript history')).toBeVisible();
    await expect(page.getByText('Contracts Finder session booted.')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Send PTY input' })).toBeDisabled();
  });

  test('PW-SES-006 supports explicit stop controls', async ({ page }) => {
    await page.goto('/sessions/session-live');
    await page.getByRole('button', { name: 'Stop graceful' }).click();
    await expect(page.getByText('Stop requested (graceful) for session-live.')).toBeVisible();
    await page.getByRole('button', { name: 'Stop force' }).click();
    await expect(page.getByText('Stop requested (force) for session-live.')).toBeVisible();
  });

  test('PW-SES-007 sends PTY input for attached sessions', async ({ page }) => {
    await page.goto('/sessions/session-live');
    await page.getByLabel('PTY input').fill('resume\n');
    await page.getByRole('button', { name: 'Send PTY input' }).click();
    await expect(page.getByText('Sent PTY input to session-live.')).toBeVisible();
  });

  test('PW-SES-008 resizes attached PTY sessions', async ({ page }) => {
    await page.goto('/sessions/session-live');
    await page.getByRole('button', { name: 'Resize 120×40' }).click();
    await expect(page.getByText('Requested 120×40 resize for session-live.')).toBeVisible();
  });

  test('PW-SES-009 surfaces replay availability after gap recovery', async ({ page }) => {
    await page.goto('/sessions/session-live');
    await expect(
      page.locator('p.page__status').filter({ hasText: 'Replay recovered 6:8.' }),
    ).toBeVisible();
  });

  test('PW-SES-010 archived sessions promote to history mode after the archive fence', async ({
    page,
  }) => {
    await page.goto('/sessions/session-failed');
    await expect(page.getByText('Viewer mode')).toBeVisible();
    await expect(page.locator('.meta-card strong').filter({ hasText: 'history' })).toBeVisible();
  });

  test('PW-SES-012 keeps a plain-text transcript alongside the PTY view', async ({
    page,
  }) => {
    await page.goto('/sessions/session-live');
    await page.getByText('Screen-reader transcript').click();
    await expect(
      page.locator('details pre').filter({ hasText: 'Connecting to supplier portal...' }),
    ).toBeVisible();
  });
});
