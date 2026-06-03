import { expect, test, type Page } from '../../e2e-playwright-compat';

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

async function decisionCalls(page: Page): Promise<DevBridgeCall[]> {
  return (await bridgeCalls(page)).filter((call) => call.cmd === 'capture_decision');
}

test.describe('decision capture', () => {
  test('PW-DEC-001 approve persists after the commit fence clears', async ({ page }) => {
    await page.goto('/process/process-1?decisionFixture=approve-after-fence&mode=full-control');
    await page.getByRole('button', { name: 'Approve' }).click();

    await expect(page.getByText('Decision: approve')).toBeVisible();
    const calls = await decisionCalls(page);
    expect(calls).toHaveLength(1);
    expect(calls[0]?.args?.reviewedBasisHash).toBe('basis-a');
    expect(calls[0]?.args?.reviewedManifestHash).toBe('artifact-process-1');
    expect(calls[0]?.args?.reviewedAtStageRunId).toBe('stage-process-1');
  });

  test('decision capture still forwards reviewed metadata when manifest output is unavailable', async ({
    page,
  }) => {
    await page.goto(
      '/process/process-metadata-only?decisionFixture=metadata-only&mode=full-control',
    );
    await page.getByRole('button', { name: 'Approve' }).click();

    await expect
      .poll(async () => {
        const calls = await decisionCalls(page);
        return calls.at(-1)?.args;
      })
      .toMatchObject({
        reviewedBasisHash: 'basis-metadata-only',
        reviewedManifestHash: 'artifact-process-metadata-only',
        reviewedAtStageRunId: 'stage-process-metadata-only',
      });
  });

  test('PW-DEC-002 watch action becomes visible after the fence', async ({ page }) => {
    await page.goto('/process/process-1?decisionFixture=watch-after-fence&mode=full-control');
    await page.getByRole('button', { name: 'Watch' }).click();

    await expect(page.getByText('Decision: watch')).toBeVisible();
  });

  test('PW-DEC-003 newer decisions supersede older intents', async ({ page }) => {
    await page.goto('/process/process-1?decisionFixture=supersede&mode=full-control');
    await page.getByRole('button', { name: 'Approve' }).click();
    await page.getByRole('button', { name: 'Reject' }).click();

    await expect(page.getByText('Decision: reject')).toBeVisible();
    const calls = await decisionCalls(page);
    expect(calls).toHaveLength(2);
    expect(calls[0]?.args?.disposition).toBe('approve');
    expect(calls[1]?.args?.disposition).toBe('reject');
    expect(calls[0]?.args?.idempotencyKey).not.toBe(calls[1]?.args?.idempotencyKey);
  });

  test('PW-DEC-004 reaffirming the same disposition still writes a new intent', async ({ page }) => {
    await page.goto('/process/process-1?decisionFixture=reaffirm-watch&mode=full-control');
    await page.getByRole('button', { name: 'Watch' }).click();

    await expect(page.getByText('Decision: watch')).toBeVisible();
    const calls = await decisionCalls(page);
    expect(calls).toHaveLength(1);
    expect(calls[0]?.args?.idempotencyKey).toBeTruthy();
  });

  test('PW-DEC-005 stale decisions surface when the basis hash changes', async ({ page }) => {
    await page.goto('/process/process-stale?decisionFixture=stale&mode=full-control');
    await expect(page.getByText('Decision: watch • stale')).toBeVisible();
  });

  test('PW-DEC-006 same-hash reruns do not mark the decision stale', async ({ page }) => {
    await page.goto('/process/process-fresh?decisionFixture=fresh&mode=full-control');
    await expect(page.getByText('Decision: watch')).toBeVisible();
    await expect(page.getByText('stale')).toHaveCount(0);
  });

  test('PW-DEC-007 mutation failure stays explicit and does not fake success', async ({ page }) => {
    await page.goto('/process/process-1?decisionFixture=mutation-failure&mode=full-control');
    await page.getByRole('button', { name: 'Approve' }).click();

    await expect(page.getByRole('alert')).toContainText('decision write failed');
    await expect(page.getByText('Decision: none')).toBeVisible();
  });

  test('PW-DEC-008 the old decision remains visible until the fence clears', async ({ page }) => {
    await page.goto('/process/process-1?decisionFixture=delayed-fence-decisions&mode=full-control');
    await expect(page.getByText('Decision: watch')).toBeVisible();
    await page.getByRole('button', { name: 'Approve' }).click();
    await expect(page.getByText('Decision: approve')).toBeVisible();
  });
});
