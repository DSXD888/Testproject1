'use client';

import { useEffect, useMemo, useState } from 'react';
import {
  ContractCardFrame,
  type PipelineState,
} from '@project/ui';
import type { ProcurementProcessRow } from '@project/contracts-schema';
import {
  listContractSummariesPage,
  type ContractListCursor,
} from '../../lib/supabase';
import {
  subscribeToContractsPage,
  type ContractsChangedUpdate,
} from '../../lib/broadcast';

const DEFAULT_VIEWER_ORG_ID = 'default';
const PAGE_SIZE = 25;
const PIPELINE_STATES = [
  'pending',
  'qualifying',
  'enriching',
  'qualified',
  'review',
  'rejected',
  'error',
] as const satisfies readonly PipelineState[];

type ListState =
  | { status: 'loading'; rows: ProcurementProcessRow[] }
  | {
      status: 'ready';
      rows: ProcurementProcessRow[];
      nextCursor: ContractListCursor | null;
      hasMore: boolean;
    }
  | { status: 'error'; rows: ProcurementProcessRow[]; message: string };

function getViewerOrgId(): string {
  return process.env.NEXT_PUBLIC_SUPABASE_ORG_ID?.trim() || DEFAULT_VIEWER_ORG_ID;
}

function isMissingViewerConfig(error: unknown): boolean {
  return (
    error instanceof Error &&
    error.message.includes('Viewer Supabase URL and anon key are required')
  );
}

function toPipelineState(stage: string): PipelineState {
  return PIPELINE_STATES.includes(stage as PipelineState)
    ? (stage as PipelineState)
    : 'pending';
}

function formatDate(value: string | null): string | null {
  if (!value) {
    return null;
  }

  return new Intl.DateTimeFormat('en-GB', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  }).format(new Date(value));
}

function buildUpdateCount(update: ContractsChangedUpdate): number {
  return new Set(update.processIds).size;
}

export default function ContractsPage() {
  const orgId = useMemo(getViewerOrgId, []);
  const [state, setState] = useState<ListState>({ status: 'loading', rows: [] });
  const [cursor, setCursor] = useState<ContractListCursor | null>(null);
  const [pendingUpdateCount, setPendingUpdateCount] = useState(0);

  useEffect(() => {
    let active = true;

    async function loadPage(): Promise<void> {
      try {
        const page = await listContractSummariesPage({ cursor, limit: PAGE_SIZE });

        if (!active) {
          return;
        }

        setState((current) => ({
          status: 'ready',
          rows: cursor ? [...current.rows, ...page.rows] : page.rows,
          nextCursor: page.nextCursor,
          hasMore: page.hasMore,
        }));
      } catch (error) {
        if (!active) {
          return;
        }

        if (isMissingViewerConfig(error)) {
          setState({
            status: 'ready',
            rows: [],
            nextCursor: null,
            hasMore: false,
          });
          return;
        }

        setState({
          status: 'error',
          rows: [],
          message: error instanceof Error ? error.message : 'Unable to load contracts',
        });
      }
    }

    void loadPage();

    return () => {
      active = false;
    };
  }, [cursor]);

  useEffect(() => {
    try {
      const subscription = subscribeToContractsPage(orgId, {
        onContractsChanged: (update) => {
          setPendingUpdateCount((current) => current + buildUpdateCount(update));
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
  }, [orgId]);

  const showPendingPill = pendingUpdateCount > 0;
  const visibleRows = state.rows;

  return (
    <section
      data-testid="contracts-list-page"
      style={{
        display: 'grid',
        gap: 20,
        padding: 24,
      }}
    >
      <header
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 16,
        }}
      >
        <div>
          <p style={{ margin: 0, color: '#59616b', fontSize: 13 }}>Daily queue</p>
          <h1 style={{ margin: '4px 0 0', fontSize: 24 }}>Contracts</h1>
        </div>
        {showPendingPill ? (
          <button
            data-testid="contracts-update-pill"
            onClick={() => {
              setPendingUpdateCount(0);
              setCursor(null);
              setState({ status: 'loading', rows: [] });
            }}
            type="button"
          >
            {pendingUpdateCount} new/updated
          </button>
        ) : null}
      </header>

      {state.status === 'loading' ? (
        <p aria-live="polite" data-testid="contracts-loading">
          Loading contracts
        </p>
      ) : null}

      {state.status === 'error' ? (
        <div data-testid="contracts-error" role="alert">
          {state.message}
        </div>
      ) : null}

      {state.status === 'ready' && visibleRows.length === 0 ? (
        <div data-testid="contracts-empty-state">No contracts in the current snapshot.</div>
      ) : null}

      {visibleRows.length > 0 ? (
        <ul
          aria-label="Opportunity queue"
          data-testid="contracts-list"
          style={{
            display: 'grid',
            gap: 16,
            listStyle: 'none',
            margin: 0,
            padding: 0,
          }}
        >
          {visibleRows.map((row) => (
            <li data-testid="contract-row" key={row.id}>
              <a
                href={`/contracts/${row.id}`}
                style={{ color: 'inherit', textDecoration: 'none' }}
              >
                <ContractCardFrame
                  summary={{
                    processId: row.id,
                    title: row.title,
                    buyerName: row.buyer_name ?? 'Unknown buyer',
                    publishedDate: formatDate(row.latest_published_at) ?? 'Unknown',
                    deadlineDate: formatDate(row.latest_deadline_at),
                    estimatedValuePence: null,
                    lotCount: 0,
                    accessRoute: 'Projection',
                    legalCategory: 'unknown',
                  }}
                  metrics={{
                    pipelineState: toPipelineState(row.current_stage),
                    pricedCoverageBps: null,
                    netQualifiedCoverageBps: null,
                    netQualifiedValueCoverageBps: null,
                    grossMarginBps: null,
                    netMarginBaseBps: null,
                    candidateCount: 0,
                    quotedCandidateCount: 0,
                  }}
                  rightRail={
                    <time dateTime={row.latest_updated_at}>
                      Updated {formatDate(row.latest_updated_at)}
                    </time>
                  }
                />
              </a>
            </li>
          ))}
        </ul>
      ) : null}

      {state.status === 'ready' ? (
        <footer>
          <button
            data-testid="contracts-next-page"
            disabled={!state.hasMore || !state.nextCursor}
            onClick={() => setCursor(state.nextCursor)}
            type="button"
          >
            Next page
          </button>
        </footer>
      ) : null}
    </section>
  );
}
