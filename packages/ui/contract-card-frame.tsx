import type { ReactElement, ReactNode } from 'react';

import type {
  GoodsLedReviewContextPayload,
  GoodsLedReviewReason,
} from '@project/contracts-schema';
import { PipelineBadge, type PipelineState } from './pipeline-badge';

export interface ContractCardFrameProps {
  summary: {
    processId: string;
    title: string;
    buyerName: string;
    publishedDate: string;
    deadlineDate: string | null;
    estimatedValuePence: number | null;
    lotCount: number;
    accessRoute: string;
    legalCategory: 'goods' | 'services' | 'works' | 'mixed' | 'unknown';
  };
  metrics: {
    pipelineState: PipelineState;
    pricedCoverageBps: number | null;
    netQualifiedCoverageBps: number | null;
    netQualifiedValueCoverageBps: number | null;
    grossMarginBps: number | null;
    netMarginBaseBps: number | null;
    labourCoverageBps?: GoodsLedReviewContextPayload['labourCoverageBps'];
    reviewReasons?: GoodsLedReviewContextPayload['reviewReasons'];
    candidateCount: number;
    quotedCandidateCount: number;
  };
  goodsLedReviewContext?: GoodsLedReviewContextPayload;
  decision?: {
    disposition: 'approve' | 'reject' | 'watch' | null;
    isStale: boolean;
    decidedAt: string | null;
  };
  rightRail?: ReactNode;
  actions?: ReactNode;
}

function formatMoney(pence: number | null): string {
  if (pence === null) {
    return 'Unknown';
  }

  return new Intl.NumberFormat('en-GB', {
    style: 'currency',
    currency: 'GBP',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(pence / 100);
}

function formatBps(bps: number | null): string {
  if (bps === null) {
    return 'N/A';
  }

  const percentage = bps / 100;
  return `${percentage.toFixed(2).replace(/\.00$/, '')}%`;
}

function renderMetric(label: string, value: string): ReactElement {
  return (
    <li className="contract-card-frame__metric">
      <span className="contract-card-frame__metric-label">{label}</span>
      <span className="contract-card-frame__metric-value">{value}</span>
    </li>
  );
}

const REVIEW_REASON_LABELS: Record<GoodsLedReviewReason, string> = {
  near_floor_cost_proxy: 'Near-floor cost proxy',
  unresolved_permit_responsibility: 'Unresolved permit responsibility',
  no_list_bespoke_flag: 'No-list/bespoke flag',
  labour_uncertainty: 'Labour uncertainty',
};

export function ContractCardFrame({
  summary,
  metrics,
  goodsLedReviewContext,
  decision,
  rightRail,
  actions,
}: ContractCardFrameProps): ReactElement {
  const reviewReasons = metrics.reviewReasons ?? goodsLedReviewContext?.reviewReasons ?? [];
  const shouldRenderReviewReasons =
    metrics.pipelineState === 'review' && reviewReasons.length > 0;

  return (
    <article
      className="contract-card-frame"
      data-process-id={summary.processId}
      data-pipeline-state={metrics.pipelineState}
    >
      <header className="contract-card-frame__header">
        <div>
          <p className="contract-card-frame__eyebrow">{summary.buyerName}</p>
          <h2>{summary.title}</h2>
          <p className="contract-card-frame__meta">
            Published {summary.publishedDate}
            {summary.deadlineDate ? ` • Deadline ${summary.deadlineDate}` : ' • No stated deadline'}
          </p>
        </div>
        <div className="contract-card-frame__header-rail">
          <PipelineBadge state={metrics.pipelineState ?? 'pending'} />
          {rightRail}
        </div>
      </header>

      <ul className="contract-card-frame__summary">
        {renderMetric('Estimated value', formatMoney(summary.estimatedValuePence))}
        {renderMetric('Lots', String(summary.lotCount))}
        {renderMetric('Access route', summary.accessRoute)}
        {renderMetric('Category', summary.legalCategory)}
      </ul>

      <ul className="contract-card-frame__metrics">
        {renderMetric('Priced coverage', formatBps(metrics.pricedCoverageBps))}
        {renderMetric('Net coverage', formatBps(metrics.netQualifiedCoverageBps))}
        {renderMetric(
          'Value-weighted gate',
          formatBps(metrics.netQualifiedValueCoverageBps),
        )}
        {renderMetric('Labour coverage', formatBps(metrics.labourCoverageBps ?? 0))}
        {renderMetric('Gross margin', formatBps(metrics.grossMarginBps))}
        {renderMetric('Net margin', formatBps(metrics.netMarginBaseBps))}
        {renderMetric('Candidates', String(metrics.candidateCount))}
        {renderMetric('Quoted', String(metrics.quotedCandidateCount))}
      </ul>

      {shouldRenderReviewReasons ? (
        <section
          aria-label="Review reasons"
          className="contract-card-frame__review-reasons"
        >
          <p className="contract-card-frame__review-reasons-title">Review reasons</p>
          <ul>
            {reviewReasons.map((reason) => (
              <li key={reason}>{REVIEW_REASON_LABELS[reason]}</li>
            ))}
          </ul>
        </section>
      ) : null}

      {decision ? (
        <section className="contract-card-frame__decision">
          <p>
            Decision: {decision.disposition ?? 'none'}
            {decision.isStale ? ' (stale)' : ''}
          </p>
          <p>Reviewed at: {decision.decidedAt ?? 'Pending review'}</p>
        </section>
      ) : null}

      {actions ? <footer className="contract-card-frame__actions">{actions}</footer> : null}
    </article>
  );
}
