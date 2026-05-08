'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ErrorSummaryList,
  FreshnessBadge,
  PipelineBadge,
  sanitizeUntrustedText,
  type ErrorSummaryItem,
  type FreshnessBadgeVM,
  type PipelineState,
} from '@project/ui';
import type {
  ErrorManifestSummaryRow,
  ViewerSyncStatusRow,
  WorkflowJobSummaryRow,
} from '@project/contracts-schema';
import {
  getStatusPageSnapshot,
  type StatusPageSnapshot,
} from '../../lib/supabase';
import {
  subscribeToStatusPage,
  type SyncUpdatedUpdate,
} from '../../lib/broadcast';

const DEFAULT_VIEWER_ORG_ID = 'default';
const STATUS_PAGE_PROJECTIONS = [
  'viewer_sync_status',
  'workflow_job_summaries',
  'error_manifest_summaries',
] as const;
const fallbackSyncVm: FreshnessBadgeVM = {
  label: 'Sync offline -- showing last known data',
  tone: 'bad',
  tooltip: 'viewer_sync_status is unavailable',
};

type StatusPageState =
  | { status: 'loading'; snapshot: StatusPageSnapshot | null }
  | { status: 'ready'; snapshot: StatusPageSnapshot }
  | { status: 'error'; snapshot: StatusPageSnapshot | null; message: string };

function getViewerOrgId(): string {
  return process.env.NEXT_PUBLIC_SUPABASE_ORG_ID?.trim() || DEFAULT_VIEWER_ORG_ID;
}

function isMissingViewerConfig(error: unknown): boolean {
  return (
    error instanceof Error &&
    error.message.includes('Viewer Supabase URL and anon key are required')
  );
}

function buildEmptyStatusSnapshot(): StatusPageSnapshot {
  return {
    syncStatus: null,
    workflowJobs: [],
    errorSummaries: [],
    loadedAt: new Date().toISOString(),
  };
}

function formatDateTime(value: string | null | undefined): string {
  if (!value) {
    return 'n/a';
  }

  const date = new Date(value);

  if (!Number.isFinite(date.getTime())) {
    return 'n/a';
  }

  return new Intl.DateTimeFormat('en-GB', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date);
}

function renderMetric(value: number | string | null | undefined): string {
  return value === null || value === undefined ? 'n/a' : String(value);
}

function toPipelineState(status: ViewerSyncStatusRow | null): PipelineState {
  switch (status?.sync_health) {
    case 'healthy':
      return 'qualified';
    case 'degraded':
      return 'review';
    case 'offline':
      return 'error';
    default:
      return 'pending';
  }
}

function renderFreshnessBadge(status: ViewerSyncStatusRow | null) {
  if (!status?.last_push_success_at) {
    return <FreshnessBadge vm={fallbackSyncVm} />;
  }

  return (
    <FreshnessBadge
      syncHealth={status.sync_health}
      lastSuccessAt={status.last_push_success_at}
    />
  );
}

function workflowJobStatus(job: WorkflowJobSummaryRow): PipelineState {
  const normalized = job.status.toLowerCase();

  if (normalized.includes('fail') || normalized.includes('error')) {
    return 'error';
  }

  if (normalized.includes('complete') || normalized.includes('success')) {
    return 'qualified';
  }

  if (normalized.includes('running') || normalized.includes('active')) {
    return 'enriching';
  }

  return 'pending';
}

function toErrorSummaryItem(row: ErrorManifestSummaryRow): ErrorSummaryItem {
  return {
    errorManifestId: row.manifest_id,
    processId: null,
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

export default function StatusPage() {
  const orgId = useMemo(getViewerOrgId, []);
  const [state, setState] = useState<StatusPageState>({
    status: 'loading',
    snapshot: null,
  });
  const [activeProjectionGeneration, setActiveProjectionGeneration] = useState<
    number | null
  >(null);
  const [pendingProjectionGeneration, setPendingProjectionGeneration] = useState<
    number | null
  >(null);

  const loadStatusSnapshot = useCallback(async (): Promise<StatusPageSnapshot> => {
    try {
      return await getStatusPageSnapshot(orgId);
    } catch (error) {
      if (isMissingViewerConfig(error)) {
        return buildEmptyStatusSnapshot();
      }

      throw error;
    }
  }, [orgId]);

  const replaceStatusSnapshot = useCallback(async (): Promise<void> => {
    try {
      const snapshot = await loadStatusSnapshot();
      setState({ status: 'ready', snapshot });
      setActiveProjectionGeneration(snapshot.syncStatus?.projection_generation ?? null);
      setPendingProjectionGeneration(null);
    } catch (error) {
      setState((current) => ({
        status: 'error',
        snapshot: current.snapshot,
        message: error instanceof Error ? error.message : 'Unable to refresh status',
      }));
    }
  }, [loadStatusSnapshot]);

  const handleSyncUpdated = useCallback(
    (update: SyncUpdatedUpdate): void => {
      if (
        activeProjectionGeneration !== null &&
        update.projectionGeneration <= activeProjectionGeneration
      ) {
        void replaceStatusSnapshot();
        return;
      }

      setPendingProjectionGeneration(update.projectionGeneration);
    },
    [activeProjectionGeneration, replaceStatusSnapshot],
  );

  useEffect(() => {
    let active = true;

    async function loadInitialSnapshot(): Promise<void> {
      try {
        const snapshot = await loadStatusSnapshot();

        if (!active) {
          return;
        }

        setState({ status: 'ready', snapshot });
        setActiveProjectionGeneration(snapshot.syncStatus?.projection_generation ?? null);
      } catch (error) {
        if (!active) {
          return;
        }

        setState({
          status: 'error',
          snapshot: null,
          message: error instanceof Error ? error.message : 'Unable to load status',
        });
      }
    }

    void loadInitialSnapshot();

    return () => {
      active = false;
    };
  }, [loadStatusSnapshot]);

  useEffect(() => {
    try {
      const subscription = subscribeToStatusPage(orgId, {
        onSyncUpdated: handleSyncUpdated,
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
  }, [handleSyncUpdated, orgId]);

  const snapshot = state.snapshot ?? buildEmptyStatusSnapshot();
  const syncStatus = snapshot.syncStatus;
  const pipelineState = toPipelineState(syncStatus);
  const errorItems = snapshot.errorSummaries.map(toErrorSummaryItem);

  return (
    <section
      data-projection={STATUS_PAGE_PROJECTIONS.join(' ')}
      data-testid="status-page"
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
          <p style={{ margin: 0, color: '#59616b', fontSize: 13 }}>
            Shared visibility
          </p>
          <h1 style={{ margin: '4px 0 0', fontSize: 24 }}>Status</h1>
          <p style={{ margin: '8px 0 0', color: '#59616b' }}>
            Projection generation {renderMetric(syncStatus?.projection_generation)} ·
            Snapshot loaded {formatDateTime(snapshot.loadedAt)}
          </p>
        </div>
        <div style={{ display: 'grid', gap: 12, justifyItems: 'end' }}>
          {renderFreshnessBadge(syncStatus)}
          {pendingProjectionGeneration !== null ? (
            <button
              data-testid="status-generation-refresh-chip"
              onClick={() => void replaceStatusSnapshot()}
              type="button"
            >
              Generation {pendingProjectionGeneration} ready
            </button>
          ) : null}
        </div>
      </header>

      {state.status === 'loading' ? (
        <p aria-live="polite" data-testid="status-loading">
          Loading status
        </p>
      ) : null}

      {state.status === 'error' ? (
        <div data-testid="status-error" role="alert">
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
          <h2>Pipeline status</h2>
          <div style={{ display: 'grid', gap: 12 }}>
            <PipelineBadge state={pipelineState} />
            <dl>
              <div>
                <dt>Sync health</dt>
                <dd>{renderMetric(syncStatus?.sync_health)}</dd>
              </div>
              <div>
                <dt>Active projection generation</dt>
                <dd>{renderMetric(activeProjectionGeneration)}</dd>
              </div>
              <div>
                <dt>Daemon version</dt>
                <dd>{renderMetric(syncStatus?.daemon_version)}</dd>
              </div>
            </dl>
          </div>
        </article>

        <article>
          <h2>Sync health</h2>
          <dl>
            <div>
              <dt>Last push success</dt>
              <dd>{formatDateTime(syncStatus?.last_push_success_at)}</dd>
            </div>
            <div>
              <dt>Last push attempt</dt>
              <dd>{formatDateTime(syncStatus?.last_push_attempt_at)}</dd>
            </div>
            <div>
              <dt>Last acked local seq</dt>
              <dd>{renderMetric(syncStatus?.last_acked_local_seq)}</dd>
            </div>
            <div>
              <dt>Pending outbox</dt>
              <dd>{renderMetric(syncStatus?.pending_outbox_count)}</dd>
            </div>
            <div>
              <dt>Schema epoch</dt>
              <dd>{renderMetric(syncStatus?.schema_epoch)}</dd>
            </div>
            <div>
              <dt>projection_generation</dt>
              <dd>{renderMetric(syncStatus?.projection_generation)}</dd>
            </div>
          </dl>
        </article>
      </section>

      <section>
        <h2>Workflow jobs</h2>
        {snapshot.workflowJobs.length > 0 ? (
          <ul>
            {snapshot.workflowJobs.map((job) => (
              <li key={job.job_id}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                  <PipelineBadge state={workflowJobStatus(job)} size="sm" />
                  <strong>{sanitizeUntrustedText(job.job_kind)}</strong>
                </div>
                <dl>
                  <div>
                    <dt>Status</dt>
                    <dd>{sanitizeUntrustedText(job.status)}</dd>
                  </div>
                  <div>
                    <dt>Skill</dt>
                    <dd>{sanitizeUntrustedText(job.skill_path ?? 'n/a')}</dd>
                  </div>
                  <div>
                    <dt>Started</dt>
                    <dd>{formatDateTime(job.started_at)}</dd>
                  </div>
                  <div>
                    <dt>Completed</dt>
                    <dd>{formatDateTime(job.completed_at)}</dd>
                  </div>
                  <div>
                    <dt>Error type</dt>
                    <dd>{sanitizeUntrustedText(job.error_type ?? 'n/a')}</dd>
                  </div>
                </dl>
              </li>
            ))}
          </ul>
        ) : (
          <p>No workflow job summaries are available in the shared projection.</p>
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
    </section>
  );
}
