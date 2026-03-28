import type { ManifestViewerProps } from '../manifest-viewer';

export const manifestViewerFixture: ManifestViewerProps = {
  manifestMarkdown: [
    '# Summary',
    'Review the [opportunity](https://example.com/opportunity) before action.',
    '# Risk Blockers',
    '- Missing landed cost evidence',
    '- Strip [bad](javascript:payload-run) links',
    '# Recommendation',
    '<img src=x onerror=payload-run>',
  ].join('\n'),
  manifestVersion: '2026.03',
  basisHash: 'basis-123',
  generatedAt: '2026-03-28T18:00:00Z',
  sections: ['Summary', 'Risk Blockers', 'Recommendation'],
  highlightBlockers: true,
};
