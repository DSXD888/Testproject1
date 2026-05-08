'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useParams } from 'next/navigation';
import {
  ErrorSummaryList,
  ManifestViewer,
  PipelineBadge,
  sanitizeUntrustedText,
  type ErrorSummaryItem,
  type PipelineState,
} from '@project/ui';
import type {
  ErrorManifestSummaryRow,
  ProcurementProcessRow,
} from '@project/contracts-schema';
import {
  getContractDetail,
  type ContractDetailSnapshot,
} from '../../../lib/supabase';
import {
  subscribeToContractDetailPage,
  type ProcessUpdatedUpdate,
} from '../../../lib/broadcast';

const DEFAULT_VIEWER_ORG_ID = 'default';
const PIPELINE_STATES = [
  'pending',
  'qualifying',
  'enriching',
  'qualified',
  'review',
  'rejected',
  'error',
] as const satisfies readonly PipelineState[];
const MAJOR_PROCESS_CHANGES = [
  'canonical_process',
  'landed_cost',
  'manifest',
] as const;

type DetailState =
  | { status: 'loading'; snapshot: ContractDetailSnapshot | null }
  | { status: 'ready'; snapshot: ContractDetailSnapshot }
  | { status: 'error'; snapshot: ContractDetailSnapshot | null; message: string };

function getViewerOrgId(): string {
  return process.env.NEXT_PUBLIC_SUPABASE_ORG_ID?.trim() || DEFAULT_VIEWER_ORG_ID;
}

function isMissingViewerConfig(error: unknown): boolean {
  return (
    error instanceof Error &&
    error.message.includes('Viewer Supabase URL and anon key are required')
  );
}

function toPipelineState(stage: string | null | undefined): PipelineState {
  return PIPELINE_STATES.includes(stage as PipelineState)
    ? (stage as PipelineState)
    : 'pending';
}

function formatDate(value: string | null | undefined): string {
  if (!value) {
    return 'Unknown';
  }

  return new Intl.DateTimeFormat('en-GB', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(value));
}

function formatMoney(pence: number | null | undefined): string {
  if (pence === null || pence === undefined) {
    return 'Unknown';
  }

  return new Intl.NumberFormat('en-GB', {
    style: 'currency',
    currency: 'GBP',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(pence / 100);
}

function formatBps(bps: number | null | undefined): string {
  if (bps === null || bps === undefined) {
    return 'N/A';
  }

  return `${(bps / 100).toFixed(2).replace(/\.00$/, '')}%`;
}

function buildEmptyProcess(processId: string): ProcurementProcessRow {
  const now = new Date().toISOString();

  return {
    id: processId,
    canonical_ocid: null,
    primary_notice_id: null,
    canonical_process_id: processId,
    current_stage: 'pending',
    buyer_name: null,
    title: 'Contract detail snapshot unavailable',
    latest_deadline_at: null,
    latest_published_at: now,
    latest_updated_at: now,
    owner_id: 'viewer',
    row_version: 0,
    last_local_seq: null,
    stage_run_id: null,
    updated_at: now,
    created_at: now,
  };
}

function buildEmptySnapshot(processId: string): ContractDetailSnapshot {
  return {
    process: buildEmptyProcess(processId),
    pipelineAssessment: null,
    candidates: [],
    quotes: [],
    pricingAssessments: [],
    landedCosts: [],
    manifest: null,
    decisionSummary: null,
    errorSummaries: [],
    workflowJobs: [],
    validationVerdicts: [],
    loadedAt: new Date().toISOString(),
  };
}

function isMajorProcessUpdate(update: ProcessUpdatedUpdate): boolean {
  return (
    update.major ||
    MAJOR_PROCESS_CHANGES.some((change) => update.changed.includes(change))
  );
}

function mergeMinorSections(
  current: ContractDetailSnapshot,
  next: ContractDetailSnapshot,
  update: ProcessUpdatedUpdate,
): ContractDetailSnapshot {
  return {
    ...current,
    loadedAt: next.loadedAt,
    quotes: update.changed.includes('quotes') ? next.quotes : current.quotes,
    pricingAssessments: update.changed.includes('quotes')
      ? next.pricingAssessments
      : current.pricingAssessments,
    workflowJobs: update.changed.includes('jobs')
      ? next.workflowJobs
      : current.workflowJobs,
    validationVerdicts: update.changed.includes('jobs')
      ? next.validationVerdicts
      : current.validationVerdicts,
    errorSummaries: update.changed.includes('errors')
      ? next.errorSummaries
      : current.errorSummaries,
  };
}

function toErrorSummaryItem(
  row: ErrorManifestSummaryRow,
  processId: string,
): ErrorSummaryItem {
  return {
    errorManifestId: row.manifest_id,
    processId,
    pipelineStage: row.job_id,
    errorType: row.error_type,
    errorMessage:
      row.error_code ??
      row.incident_signature ??
      'Redacted error summary available in shared visibility.',
    cleanupVerdict: null,
    severity: row.error_type === 'AGENT_SUSPECT_OUTPUT' ? 'critical' : 'warning',
    createdAt: row.created_at,
    resolved: false,
  };
}

export default function ContractDetailPage() {
  const params = useParams<{ id?: string | string[] }>();
  const rawProcessId = params.id;
  const processId = Array.isArray(rawProcessId)
    ? rawProcessId[0] ?? ''
    : rawProcessId ?? '';
  const orgId = useMemo(getViewerOrgId, []);
  const [state, setState] = useState<DetailState>({
    status: 'loading',
    snapshot: null,
  });
  const [pendingMajorUpdateCount, setPendingMajorUpdateCount] = useState(0);

  const loadSnapshot = useCallback(async (): Promise<ContractDetailSnapshot> => {
    try {
      return (await getContractDetail(processId, orgId)) ?? buildEmptySnapshot(processId);
    } catch (error) {
      if (isMissingViewerConfig(error)) {
        return buildEmptySnapshot(processId);
      }

      throw error;
    }
  }, [orgId, processId]);

  const replaceVisibleSnapshot = useCallback(async (): Promise<void> => {
    try {
      const snapshot = await loadSnapshot();
      setState({ status: 'ready', snapshot });
      setPendingMajorUpdateCount(0);
    } catch (error) {
      setState((current) => ({
        status: 'error',
        snapshot: current.snapshot,
        message: error instanceof Error ? error.message : 'Unable to refresh contract detail',
      }));
    }
  }, [loadSnapshot]);

  const refreshMinorSections = useCallback(
    async (update: ProcessUpdatedUpdate): Promise<void> => {
      try {
        const snapshot = await loadSnapshot();
        setState((current) => {
          if (!current.snapshot) {
            return { status: 'ready', snapshot };
          }

          return {
            status: 'ready',
            snapshot: mergeMinorSections(current.snapshot, snapshot, update),
          };
        });
      } catch (error) {
        setState((current) => ({
          status: 'error',
          snapshot: current.snapshot,
          message:
            error instanceof Error
              ? error.message
              : 'Unable to refresh contract detail section',
        }));
      }
    },
    [loadSnapshot],
  );

  useEffect(() => {
    let active = true;

    async function loadInitialSnapshot(): Promise<void> {
      try {
        const snapshot = await loadSnapshot();

        if (active) {
          setState({ status: 'ready', snapshot });
        }
      } catch (error) {
        if (active) {
          setState({
            status: 'error',
            snapshot: null,
            message:
              error instanceof Error ? error.message : 'Unable to load contract detail',
          });
        }
      }
    }

    void loadInitialSnapshot();

    return () => {
      active = false;
    };
  }, [loadSnapshot]);

  useEffect(() => {
    try {
      const subscription = subscribeToContractDetailPage(orgId, processId, {
        onProcessUpdated: (update) => {
          if (update.processId !== processId) {
            return;
          }

          if (isMajorProcessUpdate(update)) {
            setPendingMajorUpdateCount((current) => current + 1);
            return;
          }

          void refreshMinorSections(update);
        },
      });

      return () => {
        void subscription.unsubscribe();
      };
    } catch (error) {
      if (!isMissingViewerConfig(error)) {
        throw error;
      }
    }

    return undefined;
  }, [orgId, processId, refreshMinorSections]);

  const snapshot = state.snapshot;
  const process = snapshot?.process ?? buildEmptyProcess(processId);
  const pipelineState = toPipelineState(
    snapshot?.pipelineAssessment?.pipeline_state ?? process.current_stage,
  );
  const errorItems =
    snapshot?.errorSummaries.map((row) => toErrorSummaryItem(row, process.id)) ?? [];

  return (
    <section
      data-testid="contract-detail-page"
      style={{
        display: 'grid',
        gap: 20,
        padding: 24,
      }}
    >
      <header
        style={{
          display: 'flex',
          alignItems: 'flex-start',
          justifyContent: 'space-between',
          gap: 16,
        }}
      >
        <div>
          <a href="/contracts">Back to contracts</a>
          <p style={{ margin: '12px 0 0', color: '#59616b', fontSize: 13 }}>
            Opportunity detail
          </p>
          <h1 style={{ margin: '4px 0 0', fontSize: 24 }}>
            {sanitizeUntrustedText(process.title)}
          </h1>
          <p style={{ margin: '8px 0 0', color: '#59616b' }}>
            {sanitizeUntrustedText(process.buyer_name ?? 'Unknown buyer')} · Updated{' '}
            {formatDate(process.latest_updated_at)}
          </p>
        </div>
        <div style={{ display: 'grid', gap: 12, justifyItems: 'end' }}>
          <PipelineBadge state={pipelineState} />
          {pendingMajorUpdateCount > 0 ? (
            <button
              data-testid="contract-detail-refresh-chip"
              onClick={() => void replaceVisibleSnapshot()}
              type="button"
            >
              {pendingMajorUpdateCount} major update
              {pendingMajorUpdateCount === 1 ? '' : 's'} available
            </button>
          ) : null}
        </div>
      </header>

      {state.status === 'loading' ? (
        <p aria-live="polite" data-testid="contract-detail-loading">
          Loading contract detail
        </p>
      ) : null}

      {state.status === 'error' ? (
        <div data-testid="contract-detail-error" role="alert">
          {state.message}
        </div>
      ) : null}

      <section
        style={{
          display: 'grid',
          gap: 16,
          gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))',
        }}
      >
        <article>
          <h2>Decision summary</h2>
          <dl>
            <div>
              <dt>Disposition</dt>
              <dd>{snapshot?.decisionSummary?.disposition ?? 'pending'}</dd>
            </div>
            <div>
              <dt>Decision stale</dt>
              <dd>{snapshot?.decisionSummary?.decision_stale ? 'yes' : 'no'}</dd>
            </div>
            <div>
              <dt>Reviewed stage</dt>
              <dd>{snapshot?.decisionSummary?.reviewed_at_stage_run_id ?? 'n/a'}</dd>
            </div>
          </dl>
        </article>

        <article>
          <h2>Margins</h2>
          <dl>
            <div>
              <dt>Gross margin</dt>
              <dd>{formatBps(snapshot?.pipelineAssessment?.gross_margin_bps)}</dd>
            </div>
            <div>
              <dt>Net margin</dt>
              <dd>{formatBps(snapshot?.pipelineAssessment?.net_margin_bps)}</dd>
            </div>
            <div>
              <dt>Priced coverage</dt>
              <dd>{formatBps(snapshot?.pipelineAssessment?.priced_coverage_bps)}</dd>
            </div>
            <div>
              <dt>Net-qualified coverage</dt>
              <dd>{formatBps(snapshot?.pipelineAssessment?.net_qualified_coverage_bps)}</dd>
            </div>
          </dl>
        </article>
      </section>

      <section>
        <h2>Candidates</h2>
        {snapshot && snapshot.candidates.length > 0 ? (
          <ul>
            {snapshot.candidates.map((candidate) => (
              <li key={candidate.id}>
                <strong>{sanitizeUntrustedText(candidate.name)}</strong>
                <p>
                  {sanitizeUntrustedText(
                    candidate.specification_text ?? 'No specification text available.',
                  )}
                </p>
                <p>Quoteability {formatBps(candidate.quoteability_score_bps)}</p>
              </li>
            ))}
          </ul>
        ) : (
          <p>No candidates are available in the shared projection.</p>
        )}
      </section>

      <section>
        <h2>Quotes</h2>
        {snapshot && snapshot.quotes.length > 0 ? (
          <ul>
            {snapshot.quotes.map((quote) => (
              <li key={quote.id}>
                <strong>{sanitizeUntrustedText(quote.product_name)}</strong>
                <p>{sanitizeUntrustedText(quote.source_name)}</p>
                <p>
                  {formatMoney(quote.unit_price_gbp_pence)} ·{' '}
                  {sanitizeUntrustedText(quote.currency)}
                </p>
              </li>
            ))}
          </ul>
        ) : (
          <p>No supplier quotes are available in the shared projection.</p>
        )}
      </section>

      <section>
        <h2>Pricing assessments</h2>
        {snapshot && snapshot.pricingAssessments.length > 0 ? (
          <ul>
            {snapshot.pricingAssessments.map((assessment) => (
              <li key={assessment.id}>
                <strong>{formatBps(assessment.gross_margin_bps)} gross margin</strong>
                <p>
                  Revenue basis {sanitizeUntrustedText(assessment.revenue_basis_type)} ·{' '}
                  {formatMoney(assessment.expected_addressable_revenue_ex_vat_pence)}
                </p>
              </li>
            ))}
          </ul>
        ) : (
          <p>No pricing assessments are available in the shared projection.</p>
        )}
      </section>

      <section>
        <h2>Landed costs</h2>
        {snapshot && snapshot.landedCosts.length > 0 ? (
          <ul>
            {snapshot.landedCosts.map((estimate) => (
              <li key={estimate.id}>
                <strong>{formatMoney(estimate.economic_landed_cost_gbp_pence)}</strong>
                <p>
                  Net margin {formatBps(estimate.net_margin_base_bps)} · Confidence{' '}
                  {formatBps(estimate.confidence_bps)}
                </p>
              </li>
            ))}
          </ul>
        ) : (
          <p>No landed-cost estimates are available in the shared projection.</p>
        )}
      </section>

      <section>
        <h2>Manifest</h2>
        {snapshot?.manifest ? (
          <ManifestViewer
            basisHash={snapshot.manifest.manifest_basis_hash ?? 'Unavailable'}
            generatedAt={snapshot.manifest.updated_at}
            manifestMarkdown={snapshot.manifest.markdown}
            manifestVersion={snapshot.manifest.manifest_template_version}
            sections={['Decision Summary', 'Per-Candidate Detail', 'Evidence Trail']}
          />
        ) : (
          <p>No manifest is available in the shared projection.</p>
        )}
      </section>

      <section>
        <h2>Errors</h2>
        {errorItems.length > 0 ? (
          <ErrorSummaryList errors={errorItems} />
        ) : (
          <p>No redacted error summaries are available for this organisation.</p>
        )}
      </section>

      <section>
        <h2>Workflow and validation</h2>
        <dl>
          <div>
            <dt>Workflow jobs</dt>
            <dd>{snapshot?.workflowJobs.length ?? 0}</dd>
          </div>
          <div>
            <dt>Validation verdicts</dt>
            <dd>{snapshot?.validationVerdicts.length ?? 0}</dd>
          </div>
          <div>
            <dt>Snapshot loaded</dt>
            <dd>{snapshot ? formatDate(snapshot.loadedAt) : 'n/a'}</dd>
          </div>
        </dl>
      </section>
    </section>
  );
}
