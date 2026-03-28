import { expect, test } from '@playwright/test';

type InvokeRequest = {
  cmd: string;
  args?: Record<string, unknown>;
};

type DecisionDisposition = 'approve' | 'reject' | 'watch' | null;

type DecisionIntent = {
  id: string;
  processId: string;
  disposition: Exclude<DecisionDisposition, null>;
  idempotencyKey: string;
  supersededByIntentId: string | null;
};

function detailEnvelope(
  processId: string,
  decision: {
    disposition: DecisionDisposition;
    isStale: boolean;
    decidedAt: string | null;
  },
  snapshotSeq: number,
  basisHash = 'basis-a',
) {
  return {
    snapshotSeq,
    snapshotAt: '2026-03-28T21:00:00Z',
    data: {
      processId,
      canonicalProcessId: processId,
      notices: [],
      lots: [],
      candidates: [],
      quotes: [],
      landedCosts: [],
      manifest: {
        markdown: '## Summary\nDecision review packet',
        basisHash,
        generatedAt: '2026-03-28T20:00:00Z',
        sections: ['Summary'],
      },
      manifestError: null,
      coverage: {
        pricedCoverageBps: 7000,
        landedCostCoverageBps: 6500,
        netQualifiedCoverageBps: 6200,
        pricedValueCoverageBps: 7100,
        landedCostValueCoverageBps: 6100,
        netQualifiedValueCoverageBps: 5200,
        unresolvedMandatoryLots: 0,
      },
      decision,
      errors: [],
    },
  };
}

test.describe('decision capture', () => {
  test('PW-DEC-001 approve persists after the commit fence clears', async ({ page }) => {
    const intents: DecisionIntent[] = [];
    let detailReads = 0;

    await page.route('**/__tauri_invoke__', async (route) => {
      const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

      if (body.cmd === 'get_process_detail') {
        detailReads += 1;
        await route.fulfill({
          json:
            detailReads === 1
              ? detailEnvelope('process-1', { disposition: null, isStale: false, decidedAt: null }, 40)
              : detailEnvelope(
                  'process-1',
                  {
                    disposition: 'approve',
                    isStale: false,
                    decidedAt: '2026-03-28T21:00:00Z',
                  },
                  41,
                ),
        });
        return;
      }

      if (body.cmd === 'capture_decision') {
        intents.push({
          id: 'intent-1',
          processId: String(body.args?.processId),
          disposition: String(body.args?.disposition) as 'approve',
          idempotencyKey: String(body.args?.idempotencyKey),
          supersededByIntentId: null,
        });
        await route.fulfill({
          json: {
            accepted: true,
            visibleAfterSeq: 41,
            idempotencyKey: body.args?.idempotencyKey,
          },
        });
        return;
      }

      await route.fulfill({ json: { code: 'unhandled', message: body.cmd } });
    });

    await page.goto('/process/process-1');
    await page.getByRole('button', { name: 'Approve' }).click();

    await expect(page.getByText('Decision: approve')).toBeVisible();
    expect(intents).toHaveLength(1);
    expect(intents[0]?.disposition).toBe('approve');
  });

  test('PW-DEC-002 watch action becomes visible after the fence', async ({ page }) => {
    let detailReads = 0;

    await page.route('**/__tauri_invoke__', async (route) => {
      const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
      if (body.cmd === 'get_process_detail') {
        detailReads += 1;
        await route.fulfill({
          json:
            detailReads === 1
              ? detailEnvelope('process-1', { disposition: null, isStale: false, decidedAt: null }, 55)
              : detailEnvelope(
                  'process-1',
                  {
                    disposition: 'watch',
                    isStale: false,
                    decidedAt: '2026-03-28T21:02:00Z',
                  },
                  56,
                ),
        });
        return;
      }
      if (body.cmd === 'capture_decision') {
        await route.fulfill({
          json: {
            accepted: true,
            visibleAfterSeq: 56,
            idempotencyKey: body.args?.idempotencyKey,
          },
        });
        return;
      }
      await route.fulfill({ json: { code: 'unhandled', message: body.cmd } });
    });

    await page.goto('/process/process-1');
    await page.getByRole('button', { name: 'Watch' }).click();

    await expect(page.getByText('Decision: watch')).toBeVisible();
  });

  test('PW-DEC-003 newer decisions supersede older intents', async ({ page }) => {
    const intents: DecisionIntent[] = [];
    let detailReads = 0;

    await page.route('**/__tauri_invoke__', async (route) => {
      const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
      if (body.cmd === 'get_process_detail') {
        detailReads += 1;
        const disposition = detailReads >= 3 ? 'reject' : null;
        await route.fulfill({
          json: detailEnvelope(
            'process-1',
            { disposition, isStale: false, decidedAt: disposition ? '2026-03-28T21:03:00Z' : null },
            detailReads >= 3 ? 72 : 70,
          ),
        });
        return;
      }
      if (body.cmd === 'capture_decision') {
        const intentId = `intent-${intents.length + 1}`;
        if (intents.length > 0) {
          intents[intents.length - 1]!.supersededByIntentId = intentId;
        }
        intents.push({
          id: intentId,
          processId: 'process-1',
          disposition: String(body.args?.disposition) as 'approve' | 'reject',
          idempotencyKey: String(body.args?.idempotencyKey),
          supersededByIntentId: null,
        });
        await route.fulfill({
          json: {
            accepted: true,
            visibleAfterSeq: intents.length === 1 ? 71 : 72,
            idempotencyKey: body.args?.idempotencyKey,
          },
        });
        return;
      }
      await route.fulfill({ json: { code: 'unhandled', message: body.cmd } });
    });

    await page.goto('/process/process-1');
    await page.getByRole('button', { name: 'Approve' }).click();
    await page.getByRole('button', { name: 'Reject' }).click();

    await expect(page.getByText('Decision: reject')).toBeVisible();
    expect(intents).toHaveLength(2);
    expect(intents[0]?.supersededByIntentId).toBe('intent-2');
  });

  test('PW-DEC-004 reaffirming the same disposition still writes a new intent', async ({ page }) => {
    const keys: string[] = [];
    let detailReads = 0;

    await page.route('**/__tauri_invoke__', async (route) => {
      const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
      if (body.cmd === 'get_process_detail') {
        detailReads += 1;
        await route.fulfill({
          json: detailEnvelope(
            'process-1',
            {
              disposition: detailReads === 1 ? 'watch' : 'watch',
              isStale: false,
              decidedAt: detailReads === 1 ? '2026-03-28T20:55:00Z' : '2026-03-28T21:04:00Z',
            },
            detailReads === 1 ? 80 : 81,
          ),
        });
        return;
      }
      if (body.cmd === 'capture_decision') {
        keys.push(String(body.args?.idempotencyKey));
        await route.fulfill({
          json: {
            accepted: true,
            visibleAfterSeq: 81,
            idempotencyKey: body.args?.idempotencyKey,
          },
        });
        return;
      }
      await route.fulfill({ json: { code: 'unhandled', message: body.cmd } });
    });

    await page.goto('/process/process-1');
    await page.getByRole('button', { name: 'Watch' }).click();

    await expect(page.getByText('Decision: watch')).toBeVisible();
    expect(keys).toHaveLength(1);
    expect(keys[0]).toBeTruthy();
  });

  test('PW-DEC-005 stale decisions surface when the basis hash changes', async ({ page }) => {
    await page.route('**/__tauri_invoke__', async (route) => {
      const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
      if (body.cmd === 'get_process_detail') {
        await route.fulfill({
          json: detailEnvelope(
            'process-stale',
            {
              disposition: 'watch',
              isStale: true,
              decidedAt: '2026-03-28T20:00:00Z',
            },
            91,
            'basis-b',
          ),
        });
        return;
      }
      await route.fulfill({ json: { code: 'unhandled', message: body.cmd } });
    });

    await page.goto('/process/process-stale');
    await expect(page.getByText('Decision: watch • stale')).toBeVisible();
  });

  test('PW-DEC-006 same-hash reruns do not mark the decision stale', async ({ page }) => {
    await page.route('**/__tauri_invoke__', async (route) => {
      const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
      if (body.cmd === 'get_process_detail') {
        await route.fulfill({
          json: detailEnvelope(
            'process-fresh',
            {
              disposition: 'watch',
              isStale: false,
              decidedAt: '2026-03-28T20:00:00Z',
            },
            92,
            'basis-a',
          ),
        });
        return;
      }
      await route.fulfill({ json: { code: 'unhandled', message: body.cmd } });
    });

    await page.goto('/process/process-fresh');
    await expect(page.getByText('Decision: watch')).toBeVisible();
    await expect(page.getByText('stale')).toHaveCount(0);
  });

  test('PW-DEC-007 mutation failure stays explicit and does not fake success', async ({ page }) => {
    await page.route('**/__tauri_invoke__', async (route) => {
      const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
      if (body.cmd === 'get_process_detail') {
        await route.fulfill({
          json: detailEnvelope('process-1', { disposition: null, isStale: false, decidedAt: null }, 100),
        });
        return;
      }
      if (body.cmd === 'capture_decision') {
        await route.fulfill({
          json: {
            code: 'storage_unreadable',
            message: 'decision write failed',
          },
        });
        return;
      }
      await route.fulfill({ json: { code: 'unhandled', message: body.cmd } });
    });

    await page.goto('/process/process-1');
    await page.getByRole('button', { name: 'Approve' }).click();

    await expect(page.getByRole('alert')).toContainText('decision write failed');
    await expect(page.getByText('Decision: none')).toBeVisible();
  });

  test('PW-DEC-008 the old decision remains visible until the fence clears', async ({ page }) => {
    let detailReads = 0;

    await page.route('**/__tauri_invoke__', async (route) => {
      const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
      if (body.cmd === 'get_process_detail') {
        detailReads += 1;
        await route.fulfill({
          json:
            detailReads <= 2
              ? detailEnvelope(
                  'process-1',
                  {
                    disposition: 'watch',
                    isStale: false,
                    decidedAt: '2026-03-28T20:00:00Z',
                  },
                  110,
                )
              : detailEnvelope(
                  'process-1',
                  {
                    disposition: 'approve',
                    isStale: false,
                    decidedAt: '2026-03-28T21:10:00Z',
                  },
                  112,
                ),
        });
        return;
      }
      if (body.cmd === 'capture_decision') {
        await route.fulfill({
          json: {
            accepted: true,
            visibleAfterSeq: 112,
            idempotencyKey: body.args?.idempotencyKey,
          },
        });
        return;
      }
      await route.fulfill({ json: { code: 'unhandled', message: body.cmd } });
    });

    await page.goto('/process/process-1');
    await expect(page.getByText('Decision: watch')).toBeVisible();
    await page.getByRole('button', { name: 'Approve' }).click();
    await expect(page.getByText('Decision: approve')).toBeVisible();
  });
});
