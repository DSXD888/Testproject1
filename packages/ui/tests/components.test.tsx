import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import {
  ContractCardFrame,
  type ContractCardFrameProps,
} from '../contract-card-frame';
import { freshnessBadgeVmFixture } from '../fixtures/freshness-badge';
import { contractCardFrameFixture } from '../fixtures/contract-card-frame';
import {
  flaggedPipelineBadgeFixture,
  pipelineBadgeFixture,
} from '../fixtures/pipeline-badge';
import { FreshnessBadge } from '../freshness-badge';
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
});
