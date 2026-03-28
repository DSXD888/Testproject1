import type { ReactElement } from 'react';

import {
  boundUntrustedText,
  ErrorSummaryList,
  ManifestViewer,
  PipelineBadge,
  sanitizeUntrustedText,
} from '../../../../packages/ui';

type DetailState =
  | { status: 'idle' | 'loading' }
  | { status: 'error'; message: string }
  | {
      status: 'ready';
      payload: {
        snapshotSeq: number;
        snapshotAt: string;
        data: {
          processId: string;
          canonicalProcessId: string;
          notices: Array<{
            noticeId: string;
            title: string;
            buyerName: string | null;
            publishedAt: string;
            deadlineAt: string | null;
            procurementStage: string;
            summary: string | null;
          }>;
          lots: Array<{
            lotId: string;
            title: string | null;
            description: string | null;
            estimatedValuePence: number | null;
            isBidableSeparately: boolean;
          }>;
          candidates: Array<{
            candidateId: string;
            name: string;
            candidateType: string;
            specificationText: string | null;
            quoteabilityScoreBps: number | null;
          }>;
          quotes: Array<{
            quoteId: string;
            productName: string;
            sourceName: string;
            sourceUrl: string | null;
            unitPriceGbpPence: number;
            currency: string;
          }>;
          landedCosts: Array<{
            estimateId: string;
            economicLandedCostGbpPence: number;
            netMarginBaseBps: number;
            decisionGrade: string;
            confidenceBps: number;
          }>;
          manifest: {
            markdown: string;
            basisHash: string | null;
            generatedAt: string;
            sections: string[];
          } | null;
          manifestError: { code: string; message: string } | null;
          coverage: {
            pricedCoverageBps: number;
            landedCostCoverageBps: number;
            netQualifiedCoverageBps: number;
            pricedValueCoverageBps: number;
            landedCostValueCoverageBps: number;
            netQualifiedValueCoverageBps: number;
            unresolvedMandatoryLots: number;
          };
          decision: {
            disposition: 'approve' | 'reject' | 'watch' | null;
            isStale: boolean;
            decidedAt: string | null;
          };
          errors: Array<{
            errorManifestId: string;
            processId: string | null;
            pipelineStage: string | null;
            errorType: string;
            errorMessage: string;
            cleanupVerdict: 'clean' | 'forced' | 'leaked' | null;
            severity: 'critical' | 'warning' | 'info';
            createdAt: string;
            resolved: boolean;
          }>;
        };
      };
    };

const AUTO_QUALIFICATION_VALUE_FLOOR_BPS = 5000;

function formatMoney(pence: number): string {
  return new Intl.NumberFormat('en-GB', {
    style: 'currency',
    currency: 'GBP',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(pence / 100);
}

function formatBps(bps: number): string {
  return `${(bps / 100).toFixed(2)}%`;
}

function describeCoverageThreshold(netQualifiedCoverageBps: number): {
  label: string;
} {
  if (netQualifiedCoverageBps >= 7000) {
    return { label: 'Strong coverage' };
  }
  if (netQualifiedCoverageBps >= 3000) {
    return { label: 'Partial coverage -- review recommended' };
  }
  return { label: 'Low coverage -- manual review required' };
}

function isAutoQualificationReady(netQualifiedValueCoverageBps: number): boolean {
  return netQualifiedValueCoverageBps >= AUTO_QUALIFICATION_VALUE_FLOOR_BPS;
}

function renderBoundedCopy(
  value: string | null | undefined,
  fallback: string,
  maxChars = 1_200,
): { text: string; truncated: boolean } {
  const preview = boundUntrustedText(value ?? fallback, maxChars);
  return {
    text: preview.safeText,
    truncated: preview.truncated,
  };
}

export function ContractDetail({
  processId,
  state,
  onBack,
  mutationsEnabled,
  onCaptureDecision,
  decisionStatus,
}: {
  processId: string;
  state: DetailState;
  onBack: () => void;
  mutationsEnabled: boolean;
  onCaptureDecision: (processId: string, disposition: 'approve' | 'reject' | 'watch') => Promise<void>;
  decisionStatus: {
    status: 'idle' | 'saving' | 'error';
    message: string | null;
    retryable: boolean;
    pendingDisposition: 'approve' | 'reject' | 'watch' | null;
  };
}): ReactElement {
  if (state.status !== 'ready') {
    if (state.status === 'error') {
      return (
        <section className="page page--detail">
          <button onClick={onBack} type="button">
            Back to queue
          </button>
          <div className="empty-state" role="alert">
            <strong>Detail view unavailable.</strong>
            <p>{state.message}</p>
          </div>
        </section>
      );
    }

    return (
      <section className="page page--detail">
        <button onClick={onBack} type="button">
          Back to queue
        </button>
        <p className="empty-state">Loading review packet for {processId}…</p>
      </section>
    );
  }

  const { data, snapshotSeq, snapshotAt } = state.payload;
  const coverageBand = describeCoverageThreshold(data.coverage.netQualifiedCoverageBps);
  const valueGateReady = isAutoQualificationReady(
    data.coverage.netQualifiedValueCoverageBps,
  );

  return (
    <section className="page page--detail">
      <header className="page__header">
        <div>
          <button onClick={onBack} type="button">
            Back to queue
          </button>
          <p className="page__eyebrow">Review packet</p>
          <h2>{sanitizeUntrustedText(data.canonicalProcessId)}</h2>
          <p className="page__lede">
            Snapshot {snapshotSeq} at {snapshotAt}
          </p>
        </div>
        <div className="detail-header__rail">
          <PipelineBadge state="review" />
          <div className="decision-pill" data-stale={data.decision.isStale}>
            Decision: {data.decision.disposition ?? 'none'}
            {data.decision.isStale ? ' • stale' : ''}
          </div>
        </div>
      </header>

      <section className="detail-grid">
        <article className="detail-card">
          <h3>Coverage</h3>
          <dl className="metric-grid">
            <div>
              <dt>Priced coverage</dt>
              <dd>{formatBps(data.coverage.pricedCoverageBps)}</dd>
            </div>
            <div>
              <dt>Net-qualified coverage</dt>
              <dd>{formatBps(data.coverage.netQualifiedCoverageBps)}</dd>
            </div>
            <div>
              <dt>Landed-cost coverage</dt>
              <dd>{formatBps(data.coverage.landedCostCoverageBps)}</dd>
            </div>
            <div>
              <dt>Value-weighted gate</dt>
              <dd>{formatBps(data.coverage.netQualifiedValueCoverageBps)}</dd>
            </div>
            <div>
              <dt>Priced value coverage</dt>
              <dd>{formatBps(data.coverage.pricedValueCoverageBps)}</dd>
            </div>
            <div>
              <dt>Landed-cost value coverage</dt>
              <dd>{formatBps(data.coverage.landedCostValueCoverageBps)}</dd>
            </div>
            <div>
              <dt>Threshold label</dt>
              <dd>{coverageBand.label}</dd>
            </div>
            <div>
              <dt>Mandatory lots unresolved</dt>
              <dd>{data.coverage.unresolvedMandatoryLots}</dd>
            </div>
            <div>
              <dt>Auto-qualification floor</dt>
              <dd>{formatBps(AUTO_QUALIFICATION_VALUE_FLOOR_BPS)}</dd>
            </div>
          </dl>
          {!valueGateReady ? (
            <p className="warning-banner" data-testid="value-weighted-warning">
              Coverage below threshold. Strong lot-count coverage does not override a failed
              value-weighted gate.
            </p>
          ) : null}
        </article>

        <article className="detail-card">
          <h3>Decision controls</h3>
          <div className="decision-actions">
            <button
              disabled={!mutationsEnabled || decisionStatus.status === 'saving'}
              onClick={() => void onCaptureDecision(data.processId, 'approve')}
              type="button"
            >
              Approve
            </button>
            <button
              disabled={!mutationsEnabled || decisionStatus.status === 'saving'}
              onClick={() => void onCaptureDecision(data.processId, 'reject')}
              type="button"
            >
              Reject
            </button>
            <button
              disabled={!mutationsEnabled || decisionStatus.status === 'saving'}
              onClick={() => void onCaptureDecision(data.processId, 'watch')}
              type="button"
            >
              Watch
            </button>
          </div>
          <p>Reviewed at: {data.decision.decidedAt ?? 'Pending operator review'}</p>
          {decisionStatus.status === 'saving' ? (
            <p role="status">Saving decision…</p>
          ) : null}
          {decisionStatus.status === 'error' && decisionStatus.message ? (
            <p role="alert">{sanitizeUntrustedText(decisionStatus.message)}</p>
          ) : null}
          {decisionStatus.status === 'error' &&
          decisionStatus.retryable &&
          decisionStatus.pendingDisposition ? (
            <button
              onClick={() => {
                const disposition = decisionStatus.pendingDisposition;
                if (!disposition) {
                  return;
                }
                void onCaptureDecision(data.processId, disposition);
              }}
              type="button"
            >
              Retry last decision
            </button>
          ) : null}
        </article>
      </section>

      <section className="detail-grid">
        <article className="detail-card">
          <h3>Notices</h3>
          <ul className="stack-list">
            {data.notices.map((notice) => (
              <li key={notice.noticeId}>
                <strong>{sanitizeUntrustedText(notice.title)}</strong>
                <p>{sanitizeUntrustedText(notice.buyerName ?? 'Unknown buyer')}</p>
                {(() => {
                  const summary = renderBoundedCopy(
                    notice.summary,
                    'No summary provided.',
                    800,
                  );
                  return (
                    <>
                      <p>{summary.text}</p>
                      {summary.truncated ? (
                        <p role="status">Notice summary truncated for safe rendering.</p>
                      ) : null}
                    </>
                  );
                })()}
              </li>
            ))}
          </ul>
        </article>

        <article className="detail-card">
          <h3>Lots</h3>
          {data.lots.length === 0 ? (
            <p className="empty-state">No lot analysis is available for this process yet.</p>
          ) : (
            <ul className="stack-list">
              {data.lots.map((lot) => (
                <li key={lot.lotId}>
                  <strong>{sanitizeUntrustedText(lot.title ?? lot.lotId)}</strong>
                  {(() => {
                    const description = renderBoundedCopy(
                      lot.description,
                      'No description',
                      1_000,
                    );
                    return (
                      <>
                        <p>{description.text}</p>
                        {description.truncated ? (
                          <p role="status">Lot description truncated for safe rendering.</p>
                        ) : null}
                      </>
                    );
                  })()}
                  <p>
                    Estimated value:{' '}
                    {lot.estimatedValuePence === null
                      ? 'Unknown'
                      : formatMoney(lot.estimatedValuePence)}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </article>
      </section>

      <section className="detail-grid">
        <article className="detail-card">
          <h3>Candidates</h3>
          {data.candidates.length === 0 ? (
            <p className="empty-state">
              No product candidates have been extracted for this process.
            </p>
          ) : (
            <ul className="stack-list">
              {data.candidates.map((candidate) => (
                <li key={candidate.candidateId}>
                  <strong>{sanitizeUntrustedText(candidate.name)}</strong>
                  {(() => {
                    const specification = renderBoundedCopy(
                      candidate.specificationText,
                      'No specification text available.',
                      1_200,
                    );
                    return (
                      <>
                        <p>{specification.text}</p>
                        {specification.truncated ? (
                          <p role="status">
                            Candidate specification truncated for safe rendering.
                          </p>
                        ) : null}
                      </>
                    );
                  })()}
                  <p>Quoteability: {candidate.quoteabilityScoreBps ?? 0} bps</p>
                </li>
              ))}
            </ul>
          )}
        </article>

        <article className="detail-card">
          <h3>Quotes</h3>
          {data.quotes.length === 0 ? (
            <p className="empty-state">No supplier quotes are available yet.</p>
          ) : (
            <ul className="stack-list">
              {data.quotes.map((quote) => (
                <li key={quote.quoteId}>
                  <strong>{sanitizeUntrustedText(quote.productName)}</strong>
                  <p>{sanitizeUntrustedText(quote.sourceName)}</p>
                  <p>
                    {formatMoney(quote.unitPriceGbpPence)} {quote.currency}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </article>
      </section>

      <section className="detail-grid">
        <article className="detail-card">
          <h3>Landed costs</h3>
          {data.landedCosts.length === 0 ? (
            <p className="empty-state">No landed-cost estimates are available.</p>
          ) : (
            <ul className="stack-list">
              {data.landedCosts.map((estimate) => (
                <li key={estimate.estimateId}>
                  <strong>{formatMoney(estimate.economicLandedCostGbpPence)}</strong>
                  <p>Net margin: {formatBps(estimate.netMarginBaseBps)}</p>
                  <p>Confidence: {formatBps(estimate.confidenceBps)}</p>
                </li>
              ))}
            </ul>
          )}
        </article>

        <article className="detail-card">
          <h3>Manifest</h3>
          {data.manifest ? (
            <ManifestViewer
              basisHash={data.manifest.basisHash ?? 'Unavailable'}
              generatedAt={data.manifest.generatedAt}
              manifestMarkdown={data.manifest.markdown}
              manifestVersion="desktop-query"
              sections={data.manifest.sections}
            />
          ) : (
            <p className="empty-state">
              {sanitizeUntrustedText(
                data.manifestError?.message ?? 'Manifest output is not available yet.',
              )}
            </p>
          )}
        </article>
      </section>

      <section className="detail-card">
        <h3>Errors</h3>
        {data.errors.length === 0 ? (
          <p className="empty-state">No unresolved manifests are linked to this process.</p>
        ) : (
          <ErrorSummaryList errors={data.errors} />
        )}
      </section>
    </section>
  );
}
