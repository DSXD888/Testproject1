import { expect, test, type Page, type Route } from '@playwright/test';

type DevBridgeCall = {
  cmd: string;
  args?: Record<string, unknown>;
};

type BidWorkflowDetailOptions = {
  processId: string;
  bidState: 'not_started' | 'ready_to_bid' | 'draft_saved';
  readiness: {
    eligible: boolean;
    blockers?: Array<{ code: string; diagnostic: string }>;
    checkedAt?: string;
  };
  sideEffectCap?: 'REMOTE_DRAFT' | 'FORMAL_SUBMISSION';
  validatorVerdict?: 'approved' | 'rejected' | 'outage' | 'missing' | null;
  validatorMessage?: string | null;
};

type DevBridgePayload = {
  cmd?: string;
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

async function triggerBidWorkflowCalls(page: Page): Promise<DevBridgeCall[]> {
  return (await bridgeCalls(page)).filter((call) => call.cmd === 'trigger_bid_workflow');
}

async function recordOverrideCalls(page: Page): Promise<DevBridgeCall[]> {
  return (await bridgeCalls(page)).filter(
    (call) => call.cmd === 'record_bid_workflow_override',
  );
}

function readDevBridgePayload(route: Route): DevBridgePayload {
  const body = route.request().postData() ?? '{}';
  try {
    const parsed = JSON.parse(body) as unknown;
    return parsed && typeof parsed === 'object' ? (parsed as DevBridgePayload) : {};
  } catch {
    return {};
  }
}

async function installBidWorkflowFixture(
  page: Page,
  options: BidWorkflowDetailOptions,
): Promise<void> {
  await page.route('**/__tauri_invoke__', async (route: Route) => {
    const request = route.request();
    if (request.method() !== 'POST') {
      await route.fallback();
      return;
    }

    const payload = readDevBridgePayload(route);

    if (
      payload.cmd === 'get_process_detail' &&
      payload.args?.processId === options.processId
    ) {
      await route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify({
          snapshotSeq: 501,
          snapshotAt: '2026-05-23T10:00:00Z',
          data: bidWorkflowDetail(options),
        }),
      });
      return;
    }

    if (payload.cmd === 'trigger_bid_workflow') {
      await route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify({
          accepted: true,
          visibleAfterSeq: 502,
          continuationJobId: 'job-bid-workflow-001',
        }),
      });
      return;
    }

    if (payload.cmd === 'record_bid_workflow_override') {
      await route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify({
          accepted: true,
          visibleAfterSeq: 503,
        }),
      });
      return;
    }

    await route.fallback();
  });
}

function bidWorkflowDetail(options: BidWorkflowDetailOptions): Record<string, unknown> {
  const sideEffectCap = options.sideEffectCap ?? 'REMOTE_DRAFT';
  const bidScope = {
    processId: options.processId,
    lotIds: ['lot-bid-1'],
    candidateIds: ['candidate-bid-1'],
    selectedQuoteIds: ['quote-bid-1'],
    companyProfileVersion: 'company-profile-v1',
    bidPortalType: 'procontract',
    portalOpportunityRef: 'DN-START-BID-001',
    reviewRequired: true,
    decisionBasisHash: '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
    sideEffectCap,
  };

  return {
    processId: options.processId,
    canonicalProcessId: options.processId,
    pipelineState: 'qualified',
    notices: [
      {
        noticeId: 'notice-bid-1',
        sourceId: 'fts',
        sourceNoticeId: 'source-bid-1',
        title: 'Bid-ready steel widgets',
        buyerName: 'Buyer Bid',
        procurementStage: 'tender',
        summary: 'Bid workflow test opportunity',
        deadlineAt: '2026-06-30T12:00:00Z',
        publishedAt: '2026-05-01T08:00:00Z',
      },
    ],
    lots: [
      {
        lotId: 'lot-bid-1',
        noticeId: 'notice-bid-1',
        title: 'Bid Lot',
        description: 'Bid lot description',
        estimatedValuePence: 1_250_000,
        deadlineAt: '2026-06-30T12:00:00Z',
        isBidableSeparately: true,
      },
    ],
    candidates: [
      {
        candidateId: 'candidate-bid-1',
        lotId: 'lot-bid-1',
        name: 'Bid Widget',
        candidateType: 'good',
        specificationText: 'Galvanised steel',
        quoteabilityScoreBps: 8_800,
      },
    ],
    quotes: [
      {
        quoteId: 'quote-bid-1',
        productCandidateId: 'candidate-bid-1',
        productName: 'Bid Widget',
        sourceName: 'Supplier Bid',
        sourceUrl: 'https://example.invalid/bid-widget',
        unitPriceGbpPence: 1_200,
        currency: 'GBP',
      },
    ],
    landedCosts: [
      {
        estimateId: 'landed-bid-1',
        productCandidateId: 'candidate-bid-1',
        supplierQuoteId: 'quote-bid-1',
        economicLandedCostGbpPence: 1_720,
        netMarginBaseBps: 3_150,
        decisionGrade: 'screening',
        confidenceBps: 8_500,
      },
    ],
    manifest: {
      markdown: '## Summary\nReady to bid.',
      basisHash: 'basis-bid-1',
      generatedAt: '2026-05-23T09:00:00Z',
      sections: ['Summary'],
      artifactHash: 'artifact-bid-1',
      stageRunId: 'stage-bid-1',
    },
    reviewMetadata: {
      basisHash: 'basis-bid-1',
      manifestHash: 'artifact-bid-1',
      stageRunId: 'stage-bid-1',
    },
    manifestError: null,
    coverage: {
      pricedCoverageBps: 7_000,
      landedCostCoverageBps: 6_500,
      netQualifiedCoverageBps: 6_200,
      pricedValueCoverageBps: 7_100,
      landedCostValueCoverageBps: 6_100,
      netQualifiedValueCoverageBps: 6_000,
      unresolvedMandatoryLots: 0,
    },
    decision: {
      disposition: 'approve',
      isStale: false,
      decidedAt: '2026-05-23T09:30:00Z',
    },
    bidWorkflow: {
      bidState: options.bidState,
      updatedAt: '2026-05-23T09:45:00Z',
      bidScope,
      readiness: options.readiness,
      operatorId: 'operator-alinaaitova',
      companyProfile: {
        companyName: 'Sorelon',
        registrationNumber: '12345678',
        address: {
          line1: '1 Test Street',
          city: 'London',
          postcode: 'SW1A 1AA',
          country: 'GB',
        },
        primaryContact: {
          name: 'Operator',
          email: 'operator@example.invalid',
          phone: '0000000000',
          role: 'Procurement',
        },
        certifications: [],
        insurancePolicies: [],
      },
      bidDocuments: [
        {
          documentType: 'pricing_schedule',
          fileName: 'pricing.csv',
          filePath: '/tmp/pricing.csv',
          contentHash: 'sha256:pricing',
        },
      ],
      stageRunId: 'stage-bid-1',
      draftRef: options.bidState === 'draft_saved' ? 'DRAFT-BID-001' : null,
      screenshotPaths:
        options.bidState === 'draft_saved'
          ? ['screenshots/bid-001-step-1.png', 'screenshots/bid-001-step-2.png']
          : [],
      manifestHref:
        options.bidState === 'draft_saved'
          ? '/artifacts/process-bid-draft/run-manifest.md'
          : null,
      savedAt:
        options.bidState === 'draft_saved' ? '2026-05-23T09:55:00Z' : null,
      sideEffectCap,
      validatorVerdict: options.validatorVerdict ?? null,
      validatorMessage: options.validatorMessage ?? null,
    },
    errors: [],
  };
}

test.describe('bid workflow desktop controls', () => {
  test('L6-T-desktop_ui-001: Start Bid visible only for eligible ready_to_bid opportunities', async ({
    page,
  }) => {
    await installBidWorkflowFixture(page, {
      processId: 'process-bid-ready',
      bidState: 'ready_to_bid',
      readiness: {
        eligible: true,
        blockers: [],
        checkedAt: '2026-05-23T09:45:00Z',
      },
    });

    await page.goto('/process/process-bid-ready?mode=full-control');
    await page.getByRole('tab', { name: 'BidWorkflow' }).click();

    const startButton = page.getByRole('button', { name: 'Start bid' });
    await expect(startButton).toBeVisible();
    await expect(startButton).toBeEnabled();

    await startButton.click();
    await expect(page.getByTestId('start-bid-status')).toContainText('queued');
    await expect
      .poll(async () => triggerBidWorkflowCalls(page))
      .toHaveLength(1);

    const calls = await triggerBidWorkflowCalls(page);
    expect(calls[0]?.args?.processId).toBe('process-bid-ready');
    expect(calls[0]?.args?.bidScope).toMatchObject({
      processId: 'process-bid-ready',
      sideEffectCap: 'REMOTE_DRAFT',
    });
  });

  test('L6-T-desktop_ui-002: blocked Start Bid renders diagnostics and does not enqueue', async ({
    page,
  }) => {
    await installBidWorkflowFixture(page, {
      processId: 'process-bid-blocked',
      bidState: 'ready_to_bid',
      readiness: {
        eligible: false,
        blockers: [
          {
            code: 'CLASSIFICATION_AMBIGUOUS',
            diagnostic: 'Classification must be resolved before bid workflow start.',
          },
          {
            code: 'CLASSIFICATION_FRESH_UNTIL_EXPIRED',
            diagnostic: 'Classification freshness has expired; rerun L5 classification.',
          },
        ],
        checkedAt: '2026-05-23T09:50:00Z',
      },
    });

    await page.goto('/process/process-bid-blocked?mode=full-control');
    await page.getByRole('tab', { name: 'BidWorkflow' }).click();

    await expect(page.getByTestId('readiness-blockers')).toContainText(
      'CLASSIFICATION_AMBIGUOUS',
    );
    await expect(page.getByTestId('readiness-blockers')).toContainText(
      'Classification must be resolved before bid workflow start.',
    );
    await expect(page.getByTestId('readiness-blockers')).toContainText(
      'CLASSIFICATION_FRESH_UNTIL_EXPIRED',
    );
    await expect(page.getByRole('button', { name: 'Start bid' })).toBeDisabled();

    await page.getByRole('button', { name: 'Start bid' }).click({ force: true });
    expect(await triggerBidWorkflowCalls(page)).toHaveLength(0);
  });

  test('L6-T-desktop_ui-003: quote-role override requires acknowledgment and records audit payload', async ({
    page,
  }) => {
    await installBidWorkflowFixture(page, {
      processId: 'process-bid-quote-role-override',
      bidState: 'ready_to_bid',
      readiness: {
        eligible: false,
        blockers: [
          {
            code: 'QUOTE_ROLE_FLOOR_ONLY',
            diagnostic:
              'Only floor evidence is available; operator acknowledgment is required.',
          },
        ],
        checkedAt: '2026-05-23T10:05:00Z',
      },
    });

    await page.goto('/process/process-bid-quote-role-override?mode=full-control');
    await page.getByRole('tab', { name: 'BidWorkflow' }).click();
    await page.getByTestId('override-quote-role').click();

    await page.getByTestId('override-submit').click();
    await expect(page.getByTestId('override-error')).toContainText('Reason is required');

    await page
      .getByTestId('override-reason')
      .fill('Sole-source pricing; sample test bid.');
    await page.getByTestId('override-acknowledged').check();
    await page.getByTestId('override-submit').click();

    await expect(page.getByTestId('override-status')).toContainText(
      'Quote-role override recorded',
    );
    await expect.poll(async () => recordOverrideCalls(page)).toHaveLength(1);
    expect(await triggerBidWorkflowCalls(page)).toHaveLength(0);

    const [overrideCall] = await recordOverrideCalls(page);
    expect(overrideCall?.args).toMatchObject({
      processId: 'process-bid-quote-role-override',
      decisionBasisHash:
        '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
      operatorOverride: {
        kind: 'quote_role_floor_only',
        reason: 'Sole-source pricing; sample test bid.',
        recorded_by: 'operator-alinaaitova',
      },
    });
    expect(
      (overrideCall?.args?.operatorOverride as Record<string, unknown> | undefined)
        ?.recorded_at,
    ).toEqual(expect.any(String));
  });

  test('L6-T-desktop_ui-005: validation override records a reason before final submission', async ({
    page,
  }) => {
    await installBidWorkflowFixture(page, {
      processId: 'process-bid-validation-override',
      bidState: 'draft_saved',
      readiness: {
        eligible: true,
        blockers: [],
        checkedAt: '2026-05-23T10:10:00Z',
      },
      sideEffectCap: 'FORMAL_SUBMISSION',
      validatorVerdict: 'rejected',
      validatorMessage: 'Validator rejected the final portal submission packet.',
    });

    await page.goto('/process/process-bid-validation-override?mode=full-control');
    await page.getByRole('tab', { name: 'BidWorkflow' }).click();
    await page.getByTestId('override-validation').click();

    await page.getByTestId('override-submit-final').click();
    await expect(page.getByTestId('override-error')).toContainText('Reason is required');

    await page
      .getByTestId('override-reason')
      .fill('Manually verified manifest; portal confirmation captured externally.');
    await page.getByTestId('override-acknowledged').check();
    await page.getByTestId('override-submit-final').click();

    await expect(page.getByTestId('override-status')).toContainText(
      'Validation override recorded; final submission queued',
    );
    await expect.poll(async () => recordOverrideCalls(page)).toHaveLength(1);
    await expect.poll(async () => triggerBidWorkflowCalls(page)).toHaveLength(1);

    const calls = await bridgeCalls(page);
    const overrideIndex = calls.findIndex(
      (call) => call.cmd === 'record_bid_workflow_override',
    );
    const triggerIndex = calls.findIndex((call) => call.cmd === 'trigger_bid_workflow');
    expect(overrideIndex).toBeGreaterThanOrEqual(0);
    expect(triggerIndex).toBeGreaterThan(overrideIndex);

    const [overrideCall] = await recordOverrideCalls(page);
    expect(overrideCall?.args).toMatchObject({
      processId: 'process-bid-validation-override',
      sideEffectCap: 'FORMAL_SUBMISSION',
      operatorOverride: {
        kind: 'validation_gate',
        reason: 'Manually verified manifest; portal confirmation captured externally.',
        recorded_by: 'operator-alinaaitova',
      },
    });

    const [triggerCall] = await triggerBidWorkflowCalls(page);
    expect(triggerCall?.args?.bidScope).toMatchObject({
      processId: 'process-bid-validation-override',
      sideEffectCap: 'FORMAL_SUBMISSION',
    });
  });
});
