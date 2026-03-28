import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import {
  ContractCardFrame,
  type ContractCardFrameProps,
} from '../contract-card-frame';
import { freshnessBadgeVmFixture } from '../fixtures/freshness-badge';
import { contractCardFrameFixture } from '../fixtures/contract-card-frame';
import { errorSummaryFixture } from '../fixtures/error-summary-list';
import { manifestViewerFixture } from '../fixtures/manifest-viewer';
import {
  flaggedPipelineBadgeFixture,
  pipelineBadgeFixture,
} from '../fixtures/pipeline-badge';
import { ErrorSummaryList } from '../error-summary-list';
import { FreshnessBadge } from '../freshness-badge';
import { ManifestViewer } from '../manifest-viewer';
import { PipelineBadge } from '../pipeline-badge';

describe('packages/ui components', () => {
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
});
