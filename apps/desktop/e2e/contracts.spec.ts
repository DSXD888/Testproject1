import { expect, test } from '@playwright/test';

type InvokeRequest = {
  cmd: string;
  args?: Record<string, unknown>;
};

function listEnvelope(items: unknown[], nextCursor: string | null, hasMore: boolean) {
  return {
    data: {
      items,
      nextCursor,
      hasMore,
    },
    snapshotSeq: 42,
    snapshotAt: '2026-03-28T20:00:00Z',
  };
}

function makeProcess(processId: string, overrides: Partial<Record<string, unknown>> = {}) {
  return {
    processId,
    canonicalProcessId: processId,
    title: `Process ${processId}`,
    buyerName: `Buyer ${processId}`,
    publishedAt: '2026-03-28T08:00:00Z',
    deadlineAt: '2026-04-30T12:00:00Z',
    pipelineState: 'review',
    pricedCoverageBps: 7000,
    netQualifiedCoverageBps: 5000,
    netQualifiedValueCoverageBps: 4500,
    grossMarginBps: 2400,
    netMarginBaseBps: 1800,
    candidateCount: 2,
    quotedCandidateCount: 1,
    decision: {
      disposition: 'watch',
      isStale: false,
      decidedAt: '2026-03-28T09:30:00Z',
    },
    ...overrides,
  };
}

function fullDetail(processId: string) {
  return {
    data: {
      processId,
      canonicalProcessId: processId,
      notices: [
        {
          noticeId: 'notice-1',
          title: 'Steel widgets',
          buyerName: 'Buyer A',
          publishedAt: '2026-03-28T08:00:00Z',
          deadlineAt: '2026-04-30T12:00:00Z',
          procurementStage: 'tender',
          summary: 'Procurement summary',
        },
      ],
      lots: [
        {
          lotId: 'lot-1',
          title: 'Lot 1',
          description: 'Lot description',
          estimatedValuePence: 1250000,
          isBidableSeparately: true,
        },
      ],
      candidates: [
        {
          candidateId: 'candidate-1',
          name: 'Widget',
          candidateType: 'good',
          specificationText: 'Galvanised steel',
          quoteabilityScoreBps: 8800,
        },
      ],
      quotes: [
        {
          quoteId: 'quote-1',
          productName: 'Widget',
          sourceName: 'Supplier A',
          sourceUrl: 'https://example.test/quote',
          unitPriceGbpPence: 1200,
          currency: 'GBP',
        },
      ],
      landedCosts: [
        {
          estimateId: 'landed-1',
          economicLandedCostGbpPence: 1720,
          netMarginBaseBps: 3150,
          decisionGrade: 'screening',
          confidenceBps: 8500,
        },
      ],
      manifest: {
        markdown: '## Summary\nStrong fit\n\n## Risks\nMonitor freight.',
        basisHash: 'basis-1',
        generatedAt: '2026-03-28T11:00:00Z',
        sections: ['Summary', 'Risks'],
      },
      manifestError: null,
      coverage: {
        pricedCoverageBps: 7000,
        landedCostCoverageBps: 6500,
        netQualifiedCoverageBps: 6200,
        pricedValueCoverageBps: 7100,
        landedCostValueCoverageBps: 6100,
        netQualifiedValueCoverageBps: 4200,
        unresolvedMandatoryLots: 1,
      },
      decision: {
        disposition: 'watch',
        isStale: true,
        decidedAt: '2026-03-28T09:30:00Z',
      },
      errors: [
        {
          errorManifestId: 'error-1',
          processId,
          pipelineStage: 'manifest',
          errorType: 'AGENT_CLEANUP_INCOMPLETE',
          errorMessage: 'Cleanup incomplete',
          cleanupVerdict: 'leaked',
          severity: 'critical',
          createdAt: '2026-03-28T11:05:00Z',
          resolved: false,
        },
      ],
    },
    snapshotSeq: 57,
    snapshotAt: '2026-03-28T20:05:00Z',
  };
}

test.describe('contracts pages', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/__tauri_invoke__', async (route) => {
      let body: InvokeRequest;
      try {
        body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
      } catch {
        await route.fulfill({
          status: 400,
          json: {
            code: 'invalid_json',
            message: 'Invoke payload was not valid JSON',
          },
        });
        return;
      }

      if (body.cmd === 'list_processes') {
        const filters = (body.args?.filters ?? {}) as Record<string, unknown>;
        const cursor = body.args?.cursor as string | null;

        if (Array.isArray(filters.pipelineStates) && filters.pipelineStates.includes('unsupported')) {
          await route.fulfill({
            json: {
              code: 'invalid_filter',
              message: 'unsupported pipeline state',
            },
          });
          return;
        }

        if (Array.isArray(filters.sourceIds) && filters.sourceIds[0] === 'empty') {
          await route.fulfill({
            json: listEnvelope([], null, false),
          });
          return;
        }

        const staleOnly = filters.staleOnly === true;
        let items = Array.from({ length: 25 }, (_, index) =>
          makeProcess(`process-${index + 1}`),
        );

        if (cursor === 'cursor-page-2') {
          items = Array.from({ length: 10 }, (_, index) =>
            makeProcess(`process-${index + 26}`),
          );
          await route.fulfill({
            json: listEnvelope(items, null, false),
          });
          return;
        }

        if (staleOnly) {
          items = items.map((item, index) =>
            makeProcess(`stale-${index + 1}`, {
              decision: {
                disposition: 'watch',
                isStale: true,
                decidedAt: '2026-03-28T09:30:00Z',
              },
            }),
          );
        }

        if (Array.isArray(filters.decisionDispositions) && filters.decisionDispositions[0] === 'approve') {
          items = items.slice(0, 3).map((item) =>
            makeProcess(String(item.processId), {
              decision: { disposition: 'approve', isStale: false, decidedAt: '2026-03-28T09:00:00Z' },
            }),
          );
        }

        await route.fulfill({
          json: listEnvelope(items, 'cursor-page-2', true),
        });
        return;
      }

      if (body.cmd === 'get_process_detail') {
        const processId = String(body.args?.processId);
        if (processId === 'zero-lots') {
          const detail = fullDetail(processId);
          detail.data.lots = [];
          await route.fulfill({ json: detail });
          return;
        }
        if (processId === 'no-candidates') {
          const detail = fullDetail(processId);
          detail.data.candidates = [];
          detail.data.quotes = [];
          detail.data.landedCosts = [];
          detail.data.coverage = {
            ...detail.data.coverage,
            pricedCoverageBps: 0,
            landedCostCoverageBps: 0,
            netQualifiedCoverageBps: 0,
            pricedValueCoverageBps: 0,
            landedCostValueCoverageBps: 0,
            netQualifiedValueCoverageBps: 0,
          };
          await route.fulfill({ json: detail });
          return;
        }

        await route.fulfill({ json: fullDetail(processId) });
        return;
      }

      await route.fulfill({
        json: {
          code: 'unhandled',
          message: `Unhandled invoke command ${body.cmd}`,
        },
      });
    });
  });

  test('PW-CON-001 list page renders queue rows from one envelope', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByText('Contract queue')).toBeVisible();
    await expect(page.getByTestId('contract-row')).toHaveCount(25);
    await expect(page.getByTestId('contract-row').first()).toContainText('Process process-1');
    await expect(page.getByTestId('contract-row').first()).toContainText('Buyer process-1');
    await expect(page.getByTestId('contract-row').first()).toContainText('Decision: watch');
  });

  test('PW-CON-002 empty query shows explicit empty state', async ({ page }) => {
    await page.goto('/?sourceId=empty');
    await expect(page.getByTestId('contracts-empty-state')).toBeVisible();
    await expect(page.getByText('synthetic warning banner')).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Next page' })).toBeDisabled();
  });

  test('PW-CON-003 next page uses cursor without duplicates', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Next page' }).click();
    await expect(page.getByTestId('contract-row')).toHaveCount(10);
    await expect(page.getByText('Cursor: terminal-page')).toBeVisible();
    await expect(page.locator('[data-process-id="process-1"]')).toHaveCount(0);
  });

  test('PW-CON-004 refresh keeps selected row stable', async ({ page }) => {
    await page.goto('/');
    const firstCard = page.locator('.contracts-grid__button').nth(2);
    await firstCard.focus();
    await page.getByRole('button', { name: 'Refresh queue' }).click();
    await expect(firstCard).toBeFocused();
  });

  test('PW-CON-005 invalid filter values are rejected before partial render', async ({ page }) => {
    await page.goto('/?pipelineState=unsupported');
    await expect(page.getByRole('alert')).toContainText('Invalid list filter');
    await expect(page.locator('.contracts-grid li')).toHaveCount(0);
  });

  test('PW-CON-006 expired DPS distinction depends on query result, not synthetic rows', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('contract-row')).toHaveCount(25);
    await expect(page.getByText('process-26')).toHaveCount(0);
  });

  test('PW-CON-007 detail page renders the full review packet', async ({ page }) => {
    await page.goto('/process/process-1');
    await expect(page.getByText('Review packet')).toBeVisible();
    await expect(page.getByText('Steel widgets')).toBeVisible();
    await expect(page.getByText('Lot 1')).toBeVisible();
    await expect(page.getByText('Galvanised steel')).toBeVisible();
    await expect(page.getByText('£12.00 GBP')).toBeVisible();
    await expect(page.getByText('£17.20')).toBeVisible();
    await expect(page.getByText('Cleanup incomplete')).toBeVisible();
  });

  test('PW-CON-008 zero-lot process shows explicit empty-state copy', async ({ page }) => {
    await page.goto('/process/zero-lots');
    await expect(page.getByText('No lot analysis is available for this process yet.')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Approve' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Summary' })).toBeVisible();
  });

  test('PW-CON-009 empty candidates and quotes render placeholders', async ({ page }) => {
    await page.goto('/process/no-candidates');
    await expect(page.getByText('No product candidates have been extracted for this process.')).toBeVisible();
    await expect(page.getByText('No supplier quotes are available yet.')).toBeVisible();
    await expect(page.getByText('0.00%')).toHaveCount(7);
  });

  test('PW-CON-010 detail shows both lot-count and value-weighted gate outcomes', async ({ page }) => {
    await page.goto('/process/process-1');
    await expect(page.getByText('Threshold label')).toBeVisible();
    await expect(page.getByTestId('value-weighted-warning')).toContainText('Coverage below threshold');
    await expect(page.getByText('50.00%')).toBeVisible();
    await expect(page.getByText('42.00%')).toBeVisible();
  });
});
