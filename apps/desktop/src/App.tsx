import { useEffect, useMemo, useRef, useState } from 'react';
import type { ReactElement } from 'react';

import { invoke } from './lib/transport';
import { useControlPlaneState } from './hooks/use-control-plane';
import { ContractDetail } from './pages/ContractDetail';
import { DaemonStatusPage } from './pages/DaemonStatus';
import {
  type ContractListPageQuery,
  ContractList,
  type ContractListState,
  parseContractListQuery,
} from './pages/ContractList';
import { ErrorsPage } from './pages/Errors';
import SessionsPage from './pages/Sessions';
import SessionViewer from './pages/SessionViewer';

type ProcessDecision = {
  disposition: 'approve' | 'reject' | 'watch' | null;
  isStale: boolean;
  decidedAt: string | null;
};

type ProcessListItem = {
  processId: string;
  canonicalProcessId: string;
  title: string;
  buyerName: string;
  publishedAt: string;
  deadlineAt: string | null;
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
  decision: ProcessDecision;
};

type KeysetPage<T> = {
  items: T[];
  nextCursor: string | null;
  hasMore: boolean;
};

type QueryEnvelope<T> = {
  data: T;
  snapshotSeq: number;
  snapshotAt: string;
};

type NoticeVm = {
  noticeId: string;
  sourceId: string;
  sourceNoticeId: string;
  title: string;
  buyerName: string | null;
  procurementStage: string;
  summary: string | null;
  publishedAt: string;
  deadlineAt: string | null;
};

type LotVm = {
  lotId: string;
  noticeId: string;
  title: string | null;
  description: string | null;
  estimatedValuePence: number | null;
  deadlineAt: string | null;
  isBidableSeparately: boolean;
};

type CandidateVm = {
  candidateId: string;
  lotId: string;
  name: string;
  candidateType: string;
  specificationText: string | null;
  candidateValueShareBps: number | null;
  quoteabilityScoreBps: number | null;
};

type QuoteVm = {
  quoteId: string;
  productCandidateId: string | null;
  productName: string;
  sourceName: string;
  sourceUrl: string | null;
  currency: string;
  unitPriceGbpPence: number;
  priceSignalRole: string;
  shipFromCountry: string | null;
};

type LandedCostVm = {
  estimateId: string;
  productCandidateId: string;
  supplierQuoteId: string;
  economicLandedCostGbpPence: number;
  netMarginBaseBps: number;
  decisionGrade: string;
  confidenceBps: number;
};

type ErrorItem = {
  errorManifestId: string;
  processId: string | null;
  pipelineStage: string | null;
  errorType: string;
  errorMessage: string;
  cleanupVerdict: 'clean' | 'forced' | 'leaked' | null;
  severity: 'critical' | 'warning' | 'info';
  createdAt: string;
  resolved: boolean;
  agentSessionId: string | null;
};

type ProcessDetailVm = {
  processId: string;
  canonicalProcessId: string;
  notices: NoticeVm[];
  lots: LotVm[];
  candidates: CandidateVm[];
  quotes: QuoteVm[];
  landedCosts: LandedCostVm[];
  manifest: {
    markdown: string;
    basisHash: string | null;
    generatedAt: string;
    sections: string[];
  } | null;
  manifestError: { code: string; message: string; retryable: boolean } | null;
  coverage: {
    pricedCoverageBps: number;
    landedCostCoverageBps: number;
    netQualifiedCoverageBps: number;
    pricedValueCoverageBps: number;
    landedCostValueCoverageBps: number;
    netQualifiedValueCoverageBps: number;
    unresolvedMandatoryLots: number;
  };
  decision: ProcessDecision;
  errors: ErrorItem[];
};

type ScreenState<T> =
  | { status: 'idle' | 'loading' }
  | { status: 'ready'; payload: T }
  | { status: 'error'; message: string };

type DetailState = ScreenState<QueryEnvelope<ProcessDetailVm>>;

type RouteState =
  | {
      kind: 'list';
      query: ContractListPageQuery;
      validationError: string | null;
    }
  | {
      kind: 'detail';
      processId: string;
    }
  | {
      kind: 'sessions';
    }
  | {
      kind: 'session-viewer';
      sessionId: string;
      initialMode?: 'live' | 'history';
    }
  | {
      kind: 'errors';
    }
  | {
      kind: 'daemon';
    }
  | {
      kind: 'not-found';
    };

function currentRoute(pathnameOverride?: string): RouteState {
  const pathname = pathnameOverride ?? globalThis.location?.pathname ?? '/';
  const search = globalThis.location?.search ?? '';
  if (pathname.startsWith('/process/')) {
    return {
      kind: 'detail',
      processId: decodeURIComponent(pathname.replace('/process/', '')),
    };
  }
  if (pathname === '/sessions') {
    return { kind: 'sessions' };
  }
  if (pathname.startsWith('/sessions/')) {
    const params = new URLSearchParams(search);
    const initialMode = params.get('initialMode');
    return {
      kind: 'session-viewer',
      sessionId: decodeURIComponent(pathname.replace('/sessions/', '')),
      initialMode:
        initialMode === 'live' || initialMode === 'history'
          ? initialMode
          : undefined,
    };
  }
  if (pathname === '/errors') {
    return { kind: 'errors' };
  }
  if (pathname === '/daemon') {
    return { kind: 'daemon' };
  }

  if (pathname !== '/') {
    return {
      kind: 'not-found',
    };
  }

  const parsed = parseContractListQuery(search);
  return {
    kind: 'list',
    query: parsed.query,
    validationError: parsed.validationError,
  };
}

function navigate(pathname: string, search = ''): void {
  window.history.pushState({}, '', `${pathname}${search}`);
  window.dispatchEvent(new PopStateEvent('popstate'));
}

function mergeVisibleRows(
  current: ProcessListItem[],
  next: ProcessListItem[],
): ProcessListItem[] {
  if (current.length === 0) {
    return next;
  }

  const nextById = new Map(next.map((item) => [item.processId, item]));
  const preserved = current
    .map((item) => nextById.get(item.processId))
    .filter((item): item is ProcessListItem => item !== undefined);
  const preservedIds = new Set(preserved.map((item) => item.processId));
  const newcomers = next.filter((item) => !preservedIds.has(item.processId));
  return [...preserved, ...newcomers];
}

async function invokeQuery<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<QueryEnvelope<T>> {
  const response = (await invoke(cmd, args)) as QueryEnvelope<T> | { code?: string; message?: string };
  if ('data' in response) {
    return response;
  }
  throw new Error(response.message ?? `${cmd} failed`);
}

export interface AppProps {
  pathname?: string;
}

export function App({ pathname }: AppProps = {}): ReactElement {
  const [route, setRoute] = useState<RouteState>(() => currentRoute(pathname));
  const { snapshot: bootstrapSnapshot, loading: bootstrapLoading, error: bootstrapError } =
    useControlPlaneState();
  const [listState, setListState] = useState<ScreenState<KeysetPage<ProcessListItem>>>({
    status: 'idle',
  });
  const [detailState, setDetailState] = useState<DetailState>({ status: 'idle' });
  const [listEnvelopeMeta, setListEnvelopeMeta] = useState<{ snapshotSeq: number; snapshotAt: string } | null>(null);
  const [pendingCursor, setPendingCursor] = useState<string | null>(null);
  const [visibleRows, setVisibleRows] = useState<ProcessListItem[]>([]);
  const [selectedProcessId, setSelectedProcessId] = useState<string | null>(null);
  const [refreshTick, setRefreshTick] = useState(0);
  const hasHydratedList = useRef(false);

  useEffect(() => {
    const onPopState = (): void => setRoute(currentRoute(pathname));
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  }, [pathname]);

  useEffect(() => {
    if (route.kind !== 'list') {
      return;
    }

    if (route.validationError) {
      setListState({ status: 'error', message: route.validationError });
      setVisibleRows([]);
      setListEnvelopeMeta(null);
      return;
    }

    setListState({ status: 'loading' });
    void invokeQuery<KeysetPage<ProcessListItem>>('list_processes', {
      cursor: route.query.after ?? null,
      limit: route.query.limit,
      filters: {
        sourceIds: route.query.sourceId ? [route.query.sourceId] : [],
        pipelineStates: route.query.pipelineState ? [route.query.pipelineState] : [],
        decisionDispositions: route.query.decision ? [route.query.decision] : [],
        staleOnly: route.query.staleOnly === 'true',
        search: route.query.search ?? null,
      },
    })
      .then((envelope) => {
        setListEnvelopeMeta({
          snapshotSeq: envelope.snapshotSeq,
          snapshotAt: envelope.snapshotAt,
        });
        setPendingCursor(null);
        setVisibleRows((current) => {
          if (route.query.after) {
            return envelope.data.items;
          }
          if (!hasHydratedList.current) {
            hasHydratedList.current = true;
            return envelope.data.items;
          }
          return mergeVisibleRows(current, envelope.data.items);
        });
        setListState({ status: 'ready', payload: envelope.data });
        if (selectedProcessId) {
          requestAnimationFrame(() => {
            document
              .querySelector<HTMLButtonElement>(
                `.contracts-grid__button[data-process-id="${selectedProcessId}"]`,
              )
              ?.focus();
          });
        }
      })
      .catch((error: unknown) => {
        setListState({
          status: 'error',
          message: error instanceof Error ? error.message : 'List query failed',
        });
      });
  }, [refreshTick, route, selectedProcessId]);

  useEffect(() => {
    if (route.kind !== 'detail') {
      return;
    }
    setDetailState({ status: 'loading' });
    void invokeQuery<ProcessDetailVm>('get_process_detail', {
      processId: route.processId,
    })
      .then((payload) => setDetailState({ status: 'ready', payload }))
      .catch((error: unknown) =>
        setDetailState({
          status: 'error',
          message: error instanceof Error ? error.message : 'Detail query failed',
        }),
      );
  }, [route]);

  const listViewState: ContractListState = useMemo(() => {
    if (listState.status !== 'ready') {
      return listState;
    }
    return {
      status: 'ready',
      payload: {
        ...listState.payload,
        items: visibleRows,
        nextCursor: listState.payload.nextCursor,
        hasMore: listState.payload.hasMore,
        snapshotSeq: listEnvelopeMeta?.snapshotSeq ?? 0,
        snapshotAt: listEnvelopeMeta?.snapshotAt ?? '',
      },
    };
  }, [listEnvelopeMeta, listState, visibleRows]);

  const modeMessage = (() => {
    switch (bootstrapSnapshot.mode) {
      case 'full_control':
        return 'All desktop actions are available.';
      case 'control_limited':
        return 'Daemon limited — some operations unavailable.';
      case 'browse_only':
        return 'Daemon unavailable — read-only local snapshot.';
      case 'incompatible':
        return 'Protocol or schema mismatch detected.';
      case 'recovery_required':
        return 'Repair or recover storage before continuing.';
    }
  })();

  return (
    <div className="desktop-shell">
      <header className="desktop-shell__masthead">
        <div>
          <p className="desktop-shell__eyebrow">Layer 2 Desktop MVP</p>
          <h1>Procurement Opportunity Queue</h1>
        </div>
        <div className="desktop-shell__toolbar">
          <button type="button" onClick={() => navigate('/')}>
            Queue
          </button>
          <button type="button" onClick={() => navigate('/errors')}>
            Errors
          </button>
          <button type="button" onClick={() => navigate('/daemon')}>
            Daemon
          </button>
          {route.kind === 'list' ? (
            <button
              type="button"
              onClick={() => {
                setPendingCursor(null);
                setRefreshTick((value) => value + 1);
              }}
            >
              Refresh queue
            </button>
          ) : null}
        </div>
      </header>

      <div className="desktop-shell__status-banner" data-mode={bootstrapSnapshot.mode}>
        <strong>{bootstrapSnapshot.mode.replaceAll('_', ' ')}</strong>
        <span>{modeMessage}</span>
        <span>{bootstrapSnapshot.controlPlane.reasons[0] ?? 'Desktop bootstrap ready.'}</span>
        {bootstrapLoading ? <span>Refreshing bootstrap…</span> : null}
        {bootstrapError ? <span>{bootstrapError}</span> : null}
      </div>

      <main className="desktop-shell__body">
        {route.kind === 'list' ? (
          <>
            <span hidden>Contracts route</span>
            <ContractList
              query={route.query}
              state={listViewState}
              selectedProcessId={selectedProcessId}
              pendingCursor={pendingCursor}
              onOpenProcess={(processId) => {
                setSelectedProcessId(processId);
                navigate(`/process/${encodeURIComponent(processId)}`);
              }}
              onHighlightProcess={(processId) => setSelectedProcessId(processId)}
              onRequestNextPage={(nextCursor) => {
                setPendingCursor(nextCursor);
                const params = new URLSearchParams(window.location.search);
                if (nextCursor) {
                  params.set('after', nextCursor);
                } else {
                  params.delete('after');
                }
                navigate('/', params.toString() ? `?${params.toString()}` : '');
              }}
              onApplyQuery={(nextQuery) => {
                const params = new URLSearchParams();
                if (nextQuery.after) params.set('after', nextQuery.after);
                if (nextQuery.limit) params.set('limit', String(nextQuery.limit));
                if (nextQuery.search) params.set('search', nextQuery.search);
                if (nextQuery.sourceId) params.set('sourceId', nextQuery.sourceId);
                if (nextQuery.pipelineState) params.set('pipelineState', nextQuery.pipelineState);
                if (nextQuery.decision) params.set('decision', nextQuery.decision);
                if (nextQuery.staleOnly) params.set('staleOnly', nextQuery.staleOnly);
                setPendingCursor(null);
                hasHydratedList.current = false;
                navigate('/', params.toString() ? `?${params.toString()}` : '');
              }}
              onRefresh={() => setRefreshTick((value) => value + 1)}
            />
          </>
        ) : route.kind === 'detail' ? (
          <>
            <span hidden>Process route for {route.processId}</span>
            <ContractDetail
              processId={route.processId}
              state={detailState}
              onBack={() => navigate('/')}
            />
          </>
        ) : route.kind === 'sessions' ? (
          <SessionsPage />
        ) : route.kind === 'session-viewer' ? (
          <>
            <span hidden>Session route for {route.sessionId}</span>
            <SessionViewer
              sessionId={route.sessionId}
              initialMode={route.initialMode}
              onBack={() => navigate('/sessions')}
            />
          </>
        ) : route.kind === 'errors' ? (
          <>
            <span hidden>Errors route</span>
            <ErrorsPage controlPlane={bootstrapSnapshot.controlPlane} />
          </>
        ) : route.kind === 'daemon' ? (
          <>
            <span hidden>Daemon route</span>
            <DaemonStatusPage controlPlane={bootstrapSnapshot.controlPlane} />
          </>
        ) : (
          <div className="empty-state" role="status">
            Not found
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
