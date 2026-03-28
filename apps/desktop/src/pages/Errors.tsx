import type { ReactElement } from 'react';
import { useEffect, useMemo, useState } from 'react';

import {
  ErrorSummaryList,
  type ErrorSummaryItem,
} from '../../../../packages/ui/error-summary-list';
import { ManifestViewer } from '../../../../packages/ui/manifest-viewer';

import { invoke } from '../lib/transport';
import type { ControlPlaneState } from '../hooks/use-control-plane';

interface ErrorEnvelope {
  data: {
    items: ErrorSummaryItem[];
    nextCursor: string | null;
    hasMore: boolean;
  };
  snapshotSeq: number;
  snapshotAt: string;
}

interface OperatorMutationAck {
  accepted: boolean;
  visibleAfterSeq: number;
  jobId?: string;
}

const fixtureErrors: Record<string, ErrorSummaryItem[]> = {
  default: [
    {
      errorManifestId: 'err-live-1',
      processId: 'proc-001',
      pipelineStage: 'Enrich',
      errorType: 'AGENT_DAEMON_CRASH_DURING_SESSION',
      errorMessage: 'Daemon crash interrupted the supplier enrichment session.',
      cleanupVerdict: 'leaked',
      severity: 'critical',
      createdAt: '2026-03-28T09:15:00Z',
      resolved: false,
    },
    {
      errorManifestId: 'err-live-2',
      processId: 'proc-002',
      pipelineStage: 'Qualify',
      errorType: 'SOURCE_CHANGED',
      errorMessage: 'Notice page changed between qualification retries.',
      cleanupVerdict: null,
      severity: 'warning',
      createdAt: '2026-03-28T08:55:00Z',
      resolved: false,
    },
  ],
  empty: [],
  hostile: [
    {
      errorManifestId: 'err-hostile-1',
      processId: 'proc-003',
      pipelineStage: 'Extract',
      errorType: 'AGENT_SKILL_FAILED',
      errorMessage: '<svg onload=payload-run>alert(1)</svg>',
      cleanupVerdict: 'forced',
      severity: 'warning',
      createdAt: '2026-03-28T07:00:00Z',
      resolved: false,
    },
  ],
};

function currentFixture(): ErrorSummaryItem[] {
  const fixture = new URLSearchParams(globalThis.location?.search ?? '').get('errorsFixture');
  return fixtureErrors[fixture ?? 'default'] ?? fixtureErrors.default;
}

function buildManifestMarkdown(error: ErrorSummaryItem): string {
  return [
    '# Error Manifest',
    '',
    '## Summary',
    error.errorMessage,
    '',
    '## Context',
    `- Process: ${error.processId ?? 'No process linked'}`,
    `- Stage: ${error.pipelineStage ?? 'Unscoped'}`,
    `- Severity: ${error.severity}`,
    `- Cleanup: ${error.cleanupVerdict ?? 'n/a'}`,
  ].join('\n');
}

export function ErrorsPage({
  controlPlane,
}: {
  controlPlane: ControlPlaneState;
}): ReactElement {
  const [errors, setErrors] = useState<ErrorSummaryItem[]>(currentFixture());
  const [selectedId, setSelectedId] = useState<string | null>(
    currentFixture()[0]?.errorManifestId ?? null,
  );
  const [snapshotMeta, setSnapshotMeta] = useState<{
    snapshotSeq: number;
    snapshotAt: string;
    source: string;
  }>({
    snapshotSeq: 0,
    snapshotAt: '',
    source: 'fixture',
  });
  const [mutationMessage, setMutationMessage] = useState<string | null>(null);
  const [mutationPending, setMutationPending] = useState(false);

  async function reloadErrorsUntilVisible(visibleAfterSeq: number): Promise<void> {
    for (let attempt = 0; attempt < 4; attempt += 1) {
      const result = await invoke('list_errors', {
        cursor: null,
        limit: 25,
        filters: { resolved: false },
      });
      if (
        result &&
        typeof result === 'object' &&
        'data' in result &&
        'snapshotSeq' in result
      ) {
        const envelope = result as ErrorEnvelope;
        setErrors(envelope.data.items);
        setSelectedId(envelope.data.items[0]?.errorManifestId ?? null);
        setSnapshotMeta({
          snapshotSeq: envelope.snapshotSeq,
          snapshotAt: envelope.snapshotAt,
          source: 'query',
        });
        if (envelope.snapshotSeq >= visibleAfterSeq) {
          return;
        }
      }
    }
  }

  async function runErrorMutation(
    command: 'resolve_error' | 'rerun_from_error',
    errorManifestId: string,
  ): Promise<void> {
    setMutationPending(true);
    setMutationMessage(null);
    try {
      const result = await invoke(command, { errorManifestId });
      if (
        result &&
        typeof result === 'object' &&
        'accepted' in result &&
        'visibleAfterSeq' in result
      ) {
        const ack = result as OperatorMutationAck;
        await reloadErrorsUntilVisible(ack.visibleAfterSeq);
        setMutationMessage(
          command === 'rerun_from_error' && ack.jobId
            ? `Rerun accepted as ${ack.jobId}.`
            : 'Error resolution committed.',
        );
        return;
      }
      if (result && typeof result === 'object' && 'message' in result) {
        throw new Error(String((result as { message?: string }).message));
      }
      throw new Error(`${command} failed`);
    } catch (error) {
      setMutationMessage(error instanceof Error ? error.message : `${command} failed`);
    } finally {
      setMutationPending(false);
    }
  }

  useEffect(() => {
    let cancelled = false;

    void invoke('list_errors', {
      cursor: null,
      limit: 25,
      filters: { resolved: false },
    })
      .then((result) => {
        if (cancelled) {
          return;
        }
        if (
          result &&
          typeof result === 'object' &&
          'data' in result &&
          'snapshotSeq' in result
        ) {
          const envelope = result as ErrorEnvelope;
          setErrors(envelope.data.items);
          setSelectedId(envelope.data.items[0]?.errorManifestId ?? null);
          setSnapshotMeta({
            snapshotSeq: envelope.snapshotSeq,
            snapshotAt: envelope.snapshotAt,
            source: 'query',
          });
          return;
        }

        setErrors(currentFixture());
        setSelectedId(currentFixture()[0]?.errorManifestId ?? null);
      })
      .catch(() => {
        if (!cancelled) {
          setErrors(currentFixture());
          setSelectedId(currentFixture()[0]?.errorManifestId ?? null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const selectedError = useMemo(
    () => errors.find((error) => error.errorManifestId === selectedId) ?? errors[0] ?? null,
    [errors, selectedId],
  );

  return (
    <section className="desktop-page">
      <header className="desktop-page__header">
        <div>
          <p className="eyebrow">Operator queue</p>
          <h1>Errors</h1>
          <p>
            Unresolved manifests remain readable in {controlPlane.control.replace('_', ' ')} mode.
          </p>
        </div>
        <div className="meta-card">
          <span>Snapshot source</span>
          <strong>{snapshotMeta.source}</strong>
          <span>Seq {snapshotMeta.snapshotSeq}</span>
        </div>
      </header>

      {errors.length === 0 ? (
        <div className="empty-state" role="status">
          <h2>No unresolved errors</h2>
          <p>The operator queue is currently clear.</p>
        </div>
      ) : (
        <div className="two-column-layout">
          <div>
            <ErrorSummaryList
              errors={errors}
              onErrorClick={(errorManifestId) => setSelectedId(errorManifestId)}
              onResolveClick={
                controlPlane.control === 'full'
                  ? (errorManifestId) => {
                      void runErrorMutation('resolve_error', errorManifestId);
                    }
                  : null
              }
            />
          </div>

          {selectedError ? (
            <aside className="detail-card" aria-label="Error details">
              <h2>Drill-down</h2>
              <ManifestViewer
                manifestMarkdown={buildManifestMarkdown(selectedError)}
                manifestVersion="read-only-preview"
                basisHash={selectedError.errorManifestId}
                generatedAt={selectedError.createdAt}
                sections={['Summary', 'Context']}
              />
              <h3>Raw context</h3>
              <pre>{JSON.stringify(selectedError, null, 2)}</pre>
              <div className="action-row">
                <button
                  disabled={mutationPending || controlPlane.control !== 'full'}
                  onClick={() => {
                    void runErrorMutation('resolve_error', selectedError.errorManifestId);
                  }}
                  type="button"
                >
                  Resolve error
                </button>
                <button
                  disabled={mutationPending || controlPlane.control !== 'full'}
                  onClick={() => {
                    void runErrorMutation('rerun_from_error', selectedError.errorManifestId);
                  }}
                  type="button"
                >
                  Rerun from error
                </button>
              </div>
              {mutationMessage ? <p role="status">{mutationMessage}</p> : null}
            </aside>
          ) : null}
        </div>
      )}
    </section>
  );
}
