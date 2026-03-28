import { expect, test } from '@playwright/test';

type InvokeRequest = {
  cmd: string;
  args?: Record<string, unknown>;
};

function detailEnvelope(
  disposition: 'approve' | 'reject' | 'watch' | null,
  snapshotSeq: number,
) {
  return {
    snapshotSeq,
    snapshotAt: '2026-03-28T21:15:00Z',
    data: {
      processId: 'process-1',
      canonicalProcessId: 'process-1',
      notices: [],
      lots: [],
      candidates: [],
      quotes: [],
      landedCosts: [],
      manifest: {
        markdown: '## Summary\nFence test',
        basisHash: 'basis-a',
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
      decision: {
        disposition,
        isStale: false,
        decidedAt: disposition ? '2026-03-28T21:15:00Z' : null,
      },
      errors: [],
    },
  };
}

test.describe('commit fence', () => {
  test('PW-FEN-003 capture-decision keeps the old state until the fence clears', async ({ page }) => {
    let detailReads = 0;

    await page.route('**/__tauri_invoke__', async (route) => {
      const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
      if (body.cmd === 'get_process_detail') {
        detailReads += 1;
        await route.fulfill({
          json: detailReads <= 2 ? detailEnvelope('watch', 120) : detailEnvelope('approve', 122),
        });
        return;
      }
      if (body.cmd === 'capture_decision') {
        await route.fulfill({
          json: {
            accepted: true,
            visibleAfterSeq: 122,
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

  test('PW-FEN-006 rollback-like failures do not advance the visible decision', async ({ page }) => {
    await page.route('**/__tauri_invoke__', async (route) => {
      const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
      if (body.cmd === 'get_process_detail') {
        await route.fulfill({ json: detailEnvelope('watch', 130) });
        return;
      }
      if (body.cmd === 'capture_decision') {
        await route.fulfill({
          json: {
            code: 'storage_unreadable',
            message: 'rollback prevented decision persistence',
          },
        });
        return;
      }
      await route.fulfill({ json: { code: 'unhandled', message: body.cmd } });
    });

    await page.goto('/process/process-1');
    await page.getByRole('button', { name: 'Reject' }).click();
    await expect(page.getByRole('alert')).toContainText(
      'rollback prevented decision persistence',
    );
    await expect(page.getByText('Decision: watch')).toBeVisible();
  });
});
