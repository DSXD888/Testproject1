import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import type { BidScopePayload } from '@testproject1/bidding/bid-scope-types';

import * as uiBarrel from '../index';
import {
  ContractCardFrame,
  type ContractCardFrameProps,
} from '../contract-card-frame';
import { BidScopeSummary } from '../bid-scope-summary';
import { BidWorkflowStartButton } from '../bid-workflow-start-button';
import {
  BidWorkflowStatusBadge,
  type BidWorkflowState,
} from '../bid-workflow-status-badge';
import { DraftSavedReviewPanel } from '../draft-saved-review-panel';
import { freshnessBadgeVmFixture } from '../fixtures/freshness-badge';
import {
  degradedFreshnessFixture,
  healthyFreshnessFixture,
  offlineFreshnessFixture,
} from '../fixtures/freshness-badge';
import { contractCardFrameFixture } from '../fixtures/contract-card-frame';
import { errorSummaryFixture } from '../fixtures/error-summary-list';
import { uiBarrelExportFixture } from '../fixtures/index';
import { manifestViewerFixture } from '../fixtures/manifest-viewer';
import {
  flaggedPipelineBadgeFixture,
  pipelineBadgeFixture,
} from '../fixtures/pipeline-badge';
import { ErrorSummaryList } from '../error-summary-list';
import { buildSyncFreshnessVm, FreshnessBadge } from '../freshness-badge';
import { ManifestViewer } from '../manifest-viewer';
import { PipelineBadge } from '../pipeline-badge';

describe('packages/ui components', () => {
  const bidScopeFixture: BidScopePayload = {
    processId: 'process-bid-1',
    lotIds: ['lot-1', 'lot-2'],
    candidateIds: ['candidate-1'],
    selectedQuoteIds: ['quote-1'],
    companyProfileVersion: 'profile-v7',
    bidPortalType: 'procontract',
    portalOpportunityRef: 'DN123456',
    reviewRequired: true,
    decisionBasisHash: 'basis-hash-1',
    sideEffectCap: 'REMOTE_DRAFT',
  };

  it('TS-UI-002 renders the contract card slots and error state', () => {
    const props: ContractCardFrameProps = {
      ...contractCardFrameFixture,
      rightRail: <FreshnessBadge vm={freshnessBadgeVmFixture} />,
      actions: <button type="button">Open process</button>,
    };

    const markup = renderToStaticMarkup(<ContractCardFrame {...props} />);

    expect(markup).toContain('School furniture refresh');
    expect(markup).toContain('North Borough Council');
    expect(markup).toContain('Value-weighted gate');
    expect(markup).toContain('Error');
    expect(markup).toContain('2 min ago');
    expect(markup).toContain('Open process');
    expect(markup).toContain('(stale)');
  });

  it('UI-AGB-003 renders labourCoverageBps for review cards', () => {
    const props: ContractCardFrameProps = {
      ...contractCardFrameFixture,
      metrics: {
        ...contractCardFrameFixture.metrics,
        pipelineState: 'review',
      },
    };

    const markup = renderToStaticMarkup(<ContractCardFrame {...props} />);

    expect(markup).toContain('Review');
    expect(markup).toContain('Labour coverage');
    expect(markup).toContain('0%');
  });

  it('TS-UI-004 renders pipeline state with text, icon, and validation visibility rules', () => {
    const defaultMarkup = renderToStaticMarkup(<PipelineBadge {...pipelineBadgeFixture} />);
    const flaggedMarkup = renderToStaticMarkup(
      <PipelineBadge {...flaggedPipelineBadgeFixture} />,
    );

    expect(defaultMarkup).toContain('Qualified');
    expect(defaultMarkup).toContain('OK');
    expect(defaultMarkup).not.toContain('Validated');
    expect(defaultMarkup).not.toContain('Flagged');
    expect(flaggedMarkup).toContain('Review');
    expect(flaggedMarkup).toContain('RV');
    expect(flaggedMarkup).toContain('Flagged');
    expect(flaggedMarkup).toContain('FG');
    expect(flaggedMarkup).toContain('data-tone=');
  });

  it('TS-UI-005 consumes only the freshness view model shape', () => {
    const markup = renderToStaticMarkup(<FreshnessBadge vm={freshnessBadgeVmFixture} />);

    expect(markup).toContain('2 min ago');
    expect(markup).toContain('Last local refresh: 2026-03-28T18:21:00Z');
    expect(markup).toContain('data-tone="warn"');
    expect(markup).not.toContain('snapshotSeq');
    expect(markup).not.toContain('daemonHeartbeat');
  });

  it('L4-T-contracts_spec-005-007 maps sync health to three freshness states', () => {
    const healthyVm = buildSyncFreshnessVm(healthyFreshnessFixture);
    const degradedVm = buildSyncFreshnessVm(degradedFreshnessFixture);
    const offlineVm = buildSyncFreshnessVm(offlineFreshnessFixture);
    const staleVm = buildSyncFreshnessVm({
      syncHealth: 'healthy',
      lastSuccessAt: '2026-03-28T18:21:00Z',
      now: '2026-03-28T18:27:00Z',
    });

    expect(healthyVm).toMatchObject({
      label: 'Data current',
      tone: 'good',
    });
    expect(degradedVm).toMatchObject({
      label: 'Updates delayed -- data may be behind desktop',
      tone: 'warn',
    });
    expect(offlineVm).toMatchObject({
      label: 'Sync offline -- showing last known data',
      tone: 'bad',
    });
    expect(staleVm.tone).toBe('bad');

    const markup = renderToStaticMarkup(
      <FreshnessBadge {...healthyFreshnessFixture} />,
    );

    expect(markup).toContain('Data current');
    expect(markup).toContain('data-tone="good"');
    expect(markup).toContain('Last successful sync: 2026-03-28T18:21:00Z');
  });

  it('TS-UI-003 renders navigable sanitized manifest sections', () => {
    const clickedSections: string[] = [];
    const markup = renderToStaticMarkup(
      <ManifestViewer
        {...manifestViewerFixture}
        onSectionClick={(sectionId) => clickedSections.push(sectionId)}
      />,
    );

    expect(markup).toContain('Manifest version 2026.03');
    expect(markup).toContain('href="https://example.com/opportunity"');
    expect(markup).not.toContain('javascript:payload-run');
    expect(markup).not.toContain('<img src=x onerror=payload-run>');
    expect(markup).toContain('data-copy-id="summary"');
    expect(markup).toContain('data-blocker="true"');
    expect(clickedSections).toHaveLength(0);
  });

  it('UI-AGB-001 renders the Installation labour cost row from the shared payload', () => {
    const markup = renderToStaticMarkup(<ManifestViewer {...manifestViewerFixture} />);

    expect(markup).toContain('data-testid="manifest-cost-table"');
    expect(markup).toContain('data-testid="manifest-cost-row-installation-labour"');
    expect(markup).toContain('Installation labour');
    expect(markup).toContain('£960.00');
    expect(markup).toContain('£1,200.00');
    expect(markup).toContain('£1,440.00');
    expect(markup).toContain('75% (7,500 bps)');
  });

  it('UI-AGB-002 renders the separate permit and traffic-management row', () => {
    const markup = renderToStaticMarkup(<ManifestViewer {...manifestViewerFixture} />);

    expect(markup).toContain('data-testid="manifest-cost-row-permit-traffic-management"');
    expect(markup).toContain('Permits &amp; traffic management');
    expect(markup).toContain('£0.00');
    expect(markup).toContain('£250.00');
    expect(markup).toContain('£500.00');
    expect(markup).toContain('unknown');
  });

  it('keeps hostile unicode controls visible and bounds oversized payloads', () => {
    const longManifest = `${'A'.repeat(21_000)}\u202Espoofed`;
    const markup = renderToStaticMarkup(
      <ManifestViewer
        manifestMarkdown={longManifest}
        manifestVersion="2026.03"
        basisHash="basis-2"
        generatedAt="2026-03-28T20:10:00Z"
      />,
    );
    const errorMarkup = renderToStaticMarkup(
      <ErrorSummaryList
        errors={[
          {
            ...errorSummaryFixture[0],
            errorMessage: `prefix\u202Espoofed${'B'.repeat(1_500)}`,
          },
        ]}
      />,
    );

    expect(markup).toContain('Manifest truncated to the first 400 lines / 20,000 characters');
    expect(errorMarkup).toContain('Error summary truncated for safe preview.');
    expect(errorMarkup).toContain('[U+202E]');
  });

  it('TS-UI-006 renders typed error rows without requiring resolve actions', () => {
    const readOnlyMarkup = renderToStaticMarkup(
      <ErrorSummaryList errors={errorSummaryFixture} onErrorClick={() => undefined} />,
    );
    const actionableMarkup = renderToStaticMarkup(
      <ErrorSummaryList
        errors={errorSummaryFixture}
        onErrorClick={() => undefined}
        onResolveClick={() => undefined}
      />,
    );

    expect(readOnlyMarkup).toContain('SUPPLIER_MATCH_LOW_CONFIDENCE');
    expect(readOnlyMarkup).toContain('&lt;svg onload=payload-run&gt;');
    expect(readOnlyMarkup).not.toContain('>Resolve<');
    expect(readOnlyMarkup).toContain('Critical');
    expect(actionableMarkup).toContain('>Resolve<');
    expect(actionableMarkup).toContain('View details');
  });

  it('TS-UI-007 re-exports only the shared UI surface from the barrel', () => {
    for (const exportName of uiBarrelExportFixture) {
      expect(uiBarrel).toHaveProperty(exportName);
    }

    expect(uiBarrel).not.toHaveProperty('invoke');
    expect(uiBarrel).not.toHaveProperty('subscribe');
  });

  it('L6-T-desktop_ui-001 renders bid workflow states, scope review, and draft evidence', () => {
    const states: BidWorkflowState[] = [
      'not_started',
      'ready_to_bid',
      'bid_in_progress',
      'draft_saved',
      'bid_submitted',
      'bid_failed',
      'portal_state_unknown',
    ];
    const statusMarkup = renderToStaticMarkup(
      <>
        {states.map((state) => (
          <BidWorkflowStatusBadge key={state} state={state} updatedAt="2026-05-23T12:00:00Z" />
        ))}
      </>,
    );
    const scopeMarkup = renderToStaticMarkup(<BidScopeSummary scope={bidScopeFixture} />);
    const reviewMarkup = renderToStaticMarkup(
      <DraftSavedReviewPanel
        processId="process-bid-1"
        draftRef="draft-789"
        screenshotPaths={['sessions/2026/05/step-01.webp', 'sessions/2026/05/step-02.webp']}
        manifestHref="sessions/2026/05/run-manifest.md"
        onResumeDraft={() => undefined}
        onSubmitFinal={() => undefined}
        onAbandonDraft={() => undefined}
      />,
    );

    expect(statusMarkup).toContain('Not started');
    expect(statusMarkup).toContain('Ready to bid');
    expect(statusMarkup).toContain('Bid in progress');
    expect(statusMarkup).toContain('Draft saved');
    expect(statusMarkup).toContain('Bid submitted');
    expect(statusMarkup).toContain('Bid failed');
    expect(statusMarkup).toContain('Portal state unknown');
    expect(statusMarkup).toContain('data-tone="review"');
    expect(scopeMarkup).toContain('DN123456');
    expect(scopeMarkup).toContain('lot-1, lot-2');
    expect(scopeMarkup).toContain('basis-hash-1');
    expect(reviewMarkup).toContain('draft-789');
    expect(reviewMarkup).toContain('Screenshot trail');
    expect(reviewMarkup).toContain('sessions/2026/05/run-manifest.md');
    expect(reviewMarkup).toContain('Submit final');
  });

  it('L6-T-web_ui-001 keeps bid scope viewer mode bounded and start action readiness gated', () => {
    const viewerMarkup = renderToStaticMarkup(
      <BidScopeSummary scope={bidScopeFixture} viewerMode />,
    );
    const blockedMarkup = renderToStaticMarkup(
      <BidWorkflowStartButton
        readiness={{
          eligible: false,
          blockers: ['CLASSIFICATION_AMBIGUOUS', 'operator_decision_not_approved'],
          checkedAt: '2026-05-23T12:05:00Z',
        }}
        onStart={() => undefined}
      />,
    );
    const readyMarkup = renderToStaticMarkup(
      <BidWorkflowStartButton
        readiness={{ eligible: true, checkedAt: '2026-05-23T12:06:00Z' }}
        onStart={() => undefined}
      />,
    );

    expect(viewerMarkup).toContain('DN123456');
    expect(viewerMarkup).toContain('REMOTE_DRAFT');
    expect(viewerMarkup).not.toContain('lot-1');
    expect(viewerMarkup).not.toContain('candidate-1');
    expect(viewerMarkup).not.toContain('quote-1');
    expect(viewerMarkup).not.toContain('basis-hash-1');
    expect(blockedMarkup).toContain('disabled=""');
    expect(blockedMarkup).toContain('CLASSIFICATION_AMBIGUOUS');
    expect(blockedMarkup).toContain('operator_decision_not_approved');
    expect(readyMarkup).not.toContain('disabled=""');
    expect(readyMarkup).toContain('Ready to start');
  });

  it('TS-UI-008 keeps shared UI tests fixture-driven and backend-agnostic', async () => {
    expect(contractCardFrameFixture.summary.processId).toBe('proc-123');
    expect(manifestViewerFixture.sections).toEqual([
      'Summary',
      'Risk Blockers',
      'Recommendation',
    ]);
    expect(errorSummaryFixture[0]?.errorManifestId).toBe('err-1');
    expect(uiBarrel).not.toHaveProperty('invoke');
    expect(uiBarrel).not.toHaveProperty('subscribe');
  });
});
