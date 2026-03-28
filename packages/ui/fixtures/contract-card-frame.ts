import type { ContractCardFrameProps } from '../contract-card-frame';

export const contractCardFrameFixture: Omit<
  ContractCardFrameProps,
  'rightRail' | 'actions'
> = {
  summary: {
    processId: 'proc-123',
    title: 'School furniture refresh',
    buyerName: 'North Borough Council',
    publishedDate: '2026-03-20',
    deadlineDate: '2026-04-03',
    estimatedValuePence: 125000000,
    lotCount: 4,
    accessRoute: 'open',
    legalCategory: 'goods',
  },
  metrics: {
    pipelineState: 'error',
    pricedCoverageBps: 7200,
    netQualifiedCoverageBps: 6100,
    netQualifiedValueCoverageBps: 5400,
    grossMarginBps: 1800,
    netMarginBaseBps: 950,
    candidateCount: 6,
    quotedCandidateCount: 3,
  },
  decision: {
    disposition: 'watch',
    isStale: true,
    decidedAt: '2026-03-22T10:30:00Z',
  },
};
