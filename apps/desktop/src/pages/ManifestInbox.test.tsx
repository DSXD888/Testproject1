import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import { ManifestInbox, type ManifestInboxRow } from './ManifestInbox';

function fixture(): ManifestInboxRow[] {
  return [
    {
      processId: 'proc-1',
      canonicalProcessId: 'proc-1',
      title: 'DGU supply',
      buyerName: 'Council A',
      estimatedValuePence: 1_500_000,
      stoplight: 'clarify',
      suggestedAction: 'Capture clarification',
      clarificationPromptCount: 2,
      manifestBasisHash: 'hash-1',
      generatedAt: '2026-04-01T12:00:00Z',
      decision: { disposition: null, isStale: false, decidedAt: null },
    },
    {
      processId: 'proc-2',
      canonicalProcessId: 'proc-2',
      title: 'Aggregates',
      buyerName: 'Council B',
      estimatedValuePence: 8_000_000,
      stoplight: 'pursue',
      suggestedAction: 'Approve',
      clarificationPromptCount: 0,
      manifestBasisHash: 'hash-2',
      generatedAt: '2026-04-01T13:00:00Z',
      decision: { disposition: 'approve', isStale: false, decidedAt: '2026-04-01T13:30:00Z' },
    },
    {
      processId: 'proc-3',
      canonicalProcessId: 'proc-3',
      title: 'Office supplies',
      buyerName: 'Council C',
      estimatedValuePence: null,
      stoplight: 'pass',
      suggestedAction: 'Reject',
      clarificationPromptCount: 1,
      manifestBasisHash: 'hash-3',
      generatedAt: '2026-04-01T14:00:00Z',
      decision: { disposition: 'reject', isStale: true, decidedAt: '2026-04-01T14:15:00Z' },
    },
    {
      processId: 'proc-4',
      canonicalProcessId: 'proc-4',
      title: 'Catering',
      buyerName: 'Council D',
      estimatedValuePence: 5_000_000,
      stoplight: 'review',
      suggestedAction: 'Watch',
      clarificationPromptCount: 0,
      manifestBasisHash: 'hash-4',
      generatedAt: '2026-04-01T15:00:00Z',
      decision: { disposition: 'watch', isStale: false, decidedAt: '2026-04-01T15:15:00Z' },
    },
  ];
}

describe('FT.1 — ManifestInbox renders cards from list_manifests data', () => {
  it('renders one card per inbox row', () => {
    const rows = fixture();
    const html = renderToStaticMarkup(
      <ManifestInbox onOpenProcess={() => {}} seedRows={rows} />,
    );
    const matches = html.match(/data-testid="manifest-inbox-card"/g) ?? [];
    expect(matches).toHaveLength(rows.length);
  });

  it('shows the manifest count in the header status', () => {
    const rows = fixture();
    const html = renderToStaticMarkup(
      <ManifestInbox onOpenProcess={() => {}} seedRows={rows} />,
    );
    expect(html).toContain('4 manifests ready for review');
  });
});

describe('FT.2 — Stoplight badge color maps correctly', () => {
  it('applies tone-specific background colors per stoplight', () => {
    const rows = fixture();
    const html = renderToStaticMarkup(
      <ManifestInbox onOpenProcess={() => {}} seedRows={rows} />,
    );
    // clarify=yellow, pursue=green, pass=red, review=gray
    expect(html).toMatch(/data-stoplight="clarify"[^>]*background:#e0a200/);
    expect(html).toMatch(/data-stoplight="pursue"[^>]*background:#1f8a4f/);
    expect(html).toMatch(/data-stoplight="pass"[^>]*background:#c0392b/);
    expect(html).toMatch(/data-stoplight="review"[^>]*background:#5d6d7e/);
  });

  it('renders a decision pill per row, marking stale where applicable', () => {
    const rows = fixture();
    const html = renderToStaticMarkup(
      <ManifestInbox onOpenProcess={() => {}} seedRows={rows} />,
    );
    expect(html).toContain('data-disposition="none"');
    expect(html).toContain('data-disposition="approve"');
    expect(html).toContain('data-disposition="reject"');
    // Stale flag is rendered as part of the label
    expect(html).toMatch(/data-stale="true"[^>]*>Reject \(stale\)</);
  });

  it('bug_withdrawn_decision_regression_renders_withdrawn_without_stale_label', () => {
    const rows = [
      {
        ...fixture()[0],
        decision: {
          disposition: 'watch' as const,
          status: 'withdrawn',
          isStale: false,
          decidedAt: '2026-04-01T16:00:00Z',
        },
      },
    ];
    const html = renderToStaticMarkup(
      <ManifestInbox onOpenProcess={() => {}} seedRows={rows} />,
    );
    expect(html).toContain('data-status="withdrawn"');
    expect(html).toContain('>Withdrawn</span>');
    expect(html).not.toContain('(stale)');
  });
});
