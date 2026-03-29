import { expect, test, type Page } from '@playwright/test';

type DevBridgeCall = {
  cmd: string;
  args?: Record<string, unknown>;
};

async function bridgeCalls(page: Page): Promise<DevBridgeCall[]> {
  return page.evaluate(() => {
    return (
      (window as typeof window & {
        __DEV_BRIDGE_CALLS__?: Array<{ cmd: string; args?: Record<string, unknown> }>;
      }).__DEV_BRIDGE_CALLS__ ?? []
    );
  });
}

test('PW-BRD-001 surfaces transport failure and drops to browse-only mode', async ({
  page,
}) => {
  await page.goto('/errors?bridgeFixture=resolve-transport&mode=full-control');
  await page.getByRole('button', { name: 'Resolve error' }).click();

  await expect(page.getByText('daemon bridge is unavailable')).toBeVisible();
  await expect(page.getByText('Error resolution committed.')).toHaveCount(0);
  await expect(
    page.getByRole('listitem').filter({ hasText: 'AGENT_DAEMON_CRASH_DURING_SESSION' }),
  ).toHaveCount(1);
});

test('PW-BRD-002 preserves refusal reasons and only shows retry CTA for retryable failures', async ({
  page,
}) => {
  await page.goto('/errors?bridgeFixture=rerun-conflict-then-busy&mode=full-control');
  await page.getByRole('button', { name: 'Rerun from error' }).click();
  await expect(page.getByText('blocked by conflict')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Retry rerun' })).toHaveCount(0);

  await page.getByRole('button', { name: 'Rerun from error' }).click();
  await expect(page.getByText('daemon is busy')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Retry rerun' })).toBeVisible();
});

test('PW-BRD-003 surfaces invalid_params without partial session mutation', async ({
  page,
}) => {
  await page.goto('/sessions/session-live?bridgeFixture=session-input-invalid');
  await page.getByRole('button', { name: 'Send PTY input' }).click();

  await expect(page.getByRole('alert')).toContainText('invalid_params');
  await expect(page.getByText('Sent PTY input to session-live.')).toHaveCount(0);
});

test('PW-BRD-004 keeps viewer controls disabled after already-terminal no-op responses', async ({
  page,
}) => {
  await page.goto('/sessions/session-live?bridgeFixture=session-resize-terminal');
  await page.getByRole('button', { name: 'Resize 120×40' }).click();

  await expect(
    page.locator('p.page__status').filter({ hasText: 'already terminal' }),
  ).toBeVisible();
  await expect(page.getByRole('button', { name: 'Resize 120×40' })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Send PTY input' })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Stop graceful' })).toBeDisabled();
});

test('PW-BRD-005 retries lost acknowledgements with the same idempotency key', async ({
  page,
}) => {
  await page.goto('/process/process-1?bridgeFixture=decision-ack-lost&mode=full-control');
  await page.getByRole('button', { name: 'Approve' }).click();
  await expect(page.getByRole('alert')).toContainText('same idempotency key');
  await page.getByRole('button', { name: 'Retry last decision' }).click();

  await expect(page.getByText('Decision: approve')).toHaveCount(1);
  const calls = (await bridgeCalls(page)).filter((call) => call.cmd === 'capture_decision');
  expect(calls).toHaveLength(2);
  expect(calls[0]?.args?.idempotencyKey).toBe(calls[1]?.args?.idempotencyKey);
});

test('PW-BRD-006 surfaces reservation conflicts through the error queue instead of stalling', async ({
  page,
}) => {
  await page.goto('/errors?bridgeFixture=rerun-reservation-conflict&mode=full-control');
  await page.getByRole('button', { name: 'Rerun from error' }).click();

  await expect(page.getByText('file_reservation_paths conflict')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Rerun from error' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Resolve error' })).toBeVisible();
  await expect(
    page.getByRole('listitem').filter({ hasText: 'AGENT_DAEMON_CRASH_DURING_SESSION' }),
  ).toHaveCount(1);
});
