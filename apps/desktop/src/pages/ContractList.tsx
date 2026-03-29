import { useEffect, useId, useMemo, useState } from 'react';
import type { ChangeEvent, ReactElement } from 'react';

import {
  ContractCardFrame,
  FreshnessBadge,
  type FreshnessBadgeVM,
} from '../../../../packages/ui';

type ProcessRow = {
  processId: string;
  canonicalProcessId: string;
  title: string;
  buyerName: string;
  publishedAt: string;
  deadlineAt: string | null;
  estimatedValuePence: number | null;
  lotCount: number;
  accessRoute: string;
  legalCategory: 'goods' | 'services' | 'works' | 'mixed' | 'unknown';
  pipelineState:
    | 'pending'
    | 'qualifying'
    | 'enriching'
    | 'qualified'
    | 'review'
    | 'rejected'
    | 'error';
  pricedCoverageBps: number | null;
  netQualifiedCoverageBps: number | null;
  netQualifiedValueCoverageBps: number | null;
  grossMarginBps: number | null;
  netMarginBaseBps: number | null;
  candidateCount: number;
  quotedCandidateCount: number;
  decision: {
    disposition: 'approve' | 'reject' | 'watch' | null;
    isStale: boolean;
    decidedAt: string | null;
  };
};

type ListPayload = {
  items: ProcessRow[];
  nextCursor: string | null;
  hasMore: boolean;
  snapshotSeq: number;
  snapshotAt: string;
};

export type ContractListState =
  | { status: 'idle' | 'loading' }
  | { status: 'error'; message: string }
  | { status: 'ready'; payload: ListPayload };

export type ContractListPageQuery = {
  after?: string;
  limit?: 25 | 50;
  search?: string;
  sourceId?: string;
  pipelineState?: ProcessRow['pipelineState'];
  decision?: NonNullable<ProcessRow['decision']['disposition']>;
  staleOnly?: 'true' | 'false';
};

const ALLOWED_PIPELINE_STATES = new Set([
  'pending',
  'qualifying',
  'enriching',
  'qualified',
  'review',
  'rejected',
  'error',
]);

const ALLOWED_DECISIONS = new Set(['approve', 'reject', 'watch']);

export function parseContractListQuery(search: string): {
  query: ContractListPageQuery;
  validationError: string | null;
} {
  const params = new URLSearchParams(search);
  const query: ContractListPageQuery = {};

  const limit = params.get('limit');
  if (limit === '25' || limit === '50') {
    query.limit = Number(limit) as 25 | 50;
  } else if (limit !== null) {
    return { query, validationError: 'Invalid list filter: unsupported limit.' };
  }

  const after = params.get('after');
  if (after) {
    query.after = after;
  }

  const searchTerm = params.get('search');
  if (searchTerm) {
    query.search = searchTerm.trim();
  }

  const sourceId = params.get('sourceId');
  if (sourceId) {
    query.sourceId = sourceId;
  }

  const pipelineState = params.get('pipelineState');
  if (pipelineState) {
    if (!ALLOWED_PIPELINE_STATES.has(pipelineState)) {
      return { query, validationError: 'Invalid list filter: unsupported pipeline state.' };
    }
    query.pipelineState = pipelineState as ProcessRow['pipelineState'];
  }

  const decision = params.get('decision');
  if (decision) {
    if (!ALLOWED_DECISIONS.has(decision)) {
      return { query, validationError: 'Invalid list filter: unsupported decision state.' };
    }
    query.decision = decision as NonNullable<ProcessRow['decision']['disposition']>;
  }

  const staleOnly = params.get('staleOnly');
  if (staleOnly) {
    if (staleOnly !== 'true' && staleOnly !== 'false') {
      return { query, validationError: 'Invalid list filter: staleOnly must be true or false.' };
    }
    query.staleOnly = staleOnly;
  }

  return { query, validationError: null };
}

function formatDate(value: string | null): string {
  if (!value) return 'No deadline';
  return new Intl.DateTimeFormat('en-GB', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  }).format(new Date(value));
}

function buildFreshnessVm(publishedAt: string): FreshnessBadgeVM {
  const ageHours = Math.floor(
    (Date.now() - new Date(publishedAt).getTime()) / (1000 * 60 * 60),
  );
  if (ageHours < 24) {
    return {
      label: 'Fresh',
      tone: 'good',
      tooltip: 'Published within the last day.',
    };
  }
  if (ageHours < 72) {
    return {
      label: 'Aging',
      tone: 'warn',
      tooltip: 'Older than one day but still actionable.',
    };
  }
  return {
    label: 'Stale',
    tone: 'bad',
    tooltip: 'Published more than three days ago.',
  };
}

export function ContractList({
  query,
  state,
  selectedProcessId,
  pendingCursor,
  onOpenProcess,
  onHighlightProcess,
  onRequestNextPage,
  onApplyQuery,
  onRefresh,
}: {
  query: ContractListPageQuery;
  state: ContractListState;
  selectedProcessId: string | null;
  pendingCursor: string | null;
  onOpenProcess: (processId: string) => void;
  onHighlightProcess: (processId: string) => void;
  onRequestNextPage: (cursor: string | null) => void;
  onApplyQuery: (query: ContractListPageQuery) => void;
  onRefresh: () => void;
}): ReactElement {
  const searchId = useId();
  const sourceIdInputId = useId();
  const [draftSearch, setDraftSearch] = useState(query.search ?? '');
  const [draftSourceId, setDraftSourceId] = useState(query.sourceId ?? '');
  const [draftLimit, setDraftLimit] = useState<25 | 50>(query.limit ?? 25);
  const [draftPipeline, setDraftPipeline] = useState(query.pipelineState ?? '');
  const [draftDecision, setDraftDecision] = useState(query.decision ?? '');
  const [draftStaleOnly, setDraftStaleOnly] = useState(query.staleOnly === 'true');

  useEffect(() => {
    setDraftSearch(query.search ?? '');
    setDraftSourceId(query.sourceId ?? '');
    setDraftLimit(query.limit ?? 25);
    setDraftPipeline(query.pipelineState ?? '');
    setDraftDecision(query.decision ?? '');
    setDraftStaleOnly(query.staleOnly === 'true');
  }, [query]);

  const statusLabel = useMemo(() => {
    if (state.status !== 'ready') {
      return 'Snapshot pending';
    }
    return `Snapshot ${state.payload.snapshotSeq} at ${state.payload.snapshotAt}`;
  }, [state]);

  return (
    <section className="page page--contracts">
      <header className="page__header">
        <div>
          <p className="page__eyebrow">Daily queue</p>
          <h2>Contract queue</h2>
          <p className="page__lede">
            Browse the current opportunity snapshot without losing deep-link stability.
          </p>
        </div>
        <p className="page__status" role="status">
          {statusLabel}
        </p>
      </header>

      <form
        className="filters-panel"
        onSubmit={(event) => {
          event.preventDefault();
          onApplyQuery({
            limit: draftLimit,
            search: draftSearch || undefined,
            sourceId: draftSourceId || undefined,
            pipelineState: draftPipeline
              ? (draftPipeline as ContractListPageQuery['pipelineState'])
              : undefined,
            decision: draftDecision
              ? (draftDecision as ContractListPageQuery['decision'])
              : undefined,
            staleOnly: draftStaleOnly ? 'true' : undefined,
          });
        }}
      >
        <label htmlFor={searchId}>
          Search
          <input
            id={searchId}
            value={draftSearch}
            onChange={(event: ChangeEvent<HTMLInputElement>) => setDraftSearch(event.target.value)}
            placeholder="Buyer or title"
          />
        </label>
        <label htmlFor={sourceIdInputId}>
          Source ID
          <input
            id={sourceIdInputId}
            value={draftSourceId}
            onChange={(event: ChangeEvent<HTMLInputElement>) => setDraftSourceId(event.target.value)}
            placeholder="fts"
          />
        </label>
        <label>
          Page size
          <select
            value={String(draftLimit)}
            onChange={(event) => setDraftLimit(Number(event.target.value) as 25 | 50)}
          >
            <option value="25">25</option>
            <option value="50">50</option>
          </select>
        </label>
        <label>
          Pipeline
          <select
            value={draftPipeline}
            onChange={(event) => setDraftPipeline(event.target.value)}
          >
            <option value="">All</option>
            <option value="pending">Pending</option>
            <option value="qualifying">Qualifying</option>
            <option value="enriching">Enriching</option>
            <option value="qualified">Qualified</option>
            <option value="review">Review</option>
            <option value="rejected">Rejected</option>
            <option value="error">Error</option>
          </select>
        </label>
        <label>
          Decision
          <select
            value={draftDecision}
            onChange={(event) => setDraftDecision(event.target.value)}
          >
            <option value="">All</option>
            <option value="approve">Approve</option>
            <option value="reject">Reject</option>
            <option value="watch">Watch</option>
          </select>
        </label>
        <label className="filters-panel__check">
          <input
            checked={draftStaleOnly}
            onChange={(event) => setDraftStaleOnly(event.target.checked)}
            type="checkbox"
          />
          Stale decisions only
        </label>
        <button type="submit">Apply filters</button>
      </form>

      {state.status === 'loading' || state.status === 'idle' ? (
        <p className="empty-state">Loading the current queue snapshot…</p>
      ) : null}

      {state.status === 'error' ? (
        <div className="empty-state" role="alert">
          <strong>Unable to show the queue.</strong>
          <p>{state.message}</p>
        </div>
      ) : null}

      {state.status === 'ready' && state.payload.items.length === 0 ? (
        <div className="empty-state" data-testid="contracts-empty-state">
          <strong>No opportunities match this view.</strong>
          <p>
            Refine the search, filters, or stale-only toggle. This is an explicit terminal page,
            not a transport error.
          </p>
        </div>
      ) : null}

      {state.status === 'ready' && state.payload.items.length > 0 ? (
        <>
          <ul className="contracts-grid" aria-label="Opportunity queue">
            {state.payload.items.map((item) => (
              <li data-testid="contract-row" key={item.processId}>
                <button
                  className="contracts-grid__button"
                  data-process-id={item.processId}
                  data-selected={selectedProcessId === item.processId}
                  onFocus={() => onHighlightProcess(item.processId)}
                  onClick={() => onOpenProcess(item.canonicalProcessId)}
                  type="button"
                >
                  <ContractCardFrame
                    summary={{
                      processId: item.processId,
                      title: item.title,
                      buyerName: item.buyerName,
                      publishedDate: formatDate(item.publishedAt),
                      deadlineDate: formatDate(item.deadlineAt),
                      estimatedValuePence: item.estimatedValuePence,
                      lotCount: item.lotCount,
                      accessRoute: item.accessRoute,
                      legalCategory: item.legalCategory,
                    }}
                    metrics={{
                      pipelineState: item.pipelineState,
                      pricedCoverageBps: item.pricedCoverageBps,
                      netQualifiedCoverageBps: item.netQualifiedCoverageBps,
                      netQualifiedValueCoverageBps: item.netQualifiedValueCoverageBps,
                      grossMarginBps: item.grossMarginBps,
                      netMarginBaseBps: item.netMarginBaseBps,
                      candidateCount: item.candidateCount,
                      quotedCandidateCount: item.quotedCandidateCount,
                    }}
                    decision={item.decision}
                    rightRail={<FreshnessBadge vm={buildFreshnessVm(item.publishedAt)} />}
                  />
                </button>
              </li>
            ))}
          </ul>

        </>
      ) : null}

      {state.status === 'ready' ? (
        <footer className="pagination-bar">
          <p data-testid="cursor-state">
            Cursor: {state.payload.nextCursor ?? pendingCursor ?? 'terminal-page'}
          </p>
          <button onClick={onRefresh} type="button">
            Deliberate refresh
          </button>
          <button
            disabled={!state.payload.hasMore || !state.payload.nextCursor}
            onClick={() => onRequestNextPage(state.payload.nextCursor)}
            type="button"
          >
            Next page
          </button>
        </footer>
      ) : null}
    </section>
  );
}
