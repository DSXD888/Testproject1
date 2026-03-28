import type { ErrorSummaryItem } from '../error-summary-list';

export const errorSummaryFixture: ErrorSummaryItem[] = [
  {
    errorManifestId: 'err-1',
    processId: 'proc-1',
    pipelineStage: 'enrichment',
    errorType: 'SUPPLIER_MATCH_LOW_CONFIDENCE',
    errorMessage: '<svg onload=payload-run>Supplier match confidence too low</svg>',
    cleanupVerdict: 'forced',
    severity: 'warning',
    createdAt: '2026-03-28T18:10:00Z',
    resolved: false,
  },
  {
    errorManifestId: 'err-2',
    processId: null,
    pipelineStage: null,
    errorType: 'AGENT_CLEANUP_INCOMPLETE',
    errorMessage: 'Cleanup leaked child process',
    cleanupVerdict: 'leaked',
    severity: 'critical',
    createdAt: '2026-03-28T18:12:00Z',
    resolved: true,
    resolvedAt: '2026-03-28T18:20:00Z',
  },
];
