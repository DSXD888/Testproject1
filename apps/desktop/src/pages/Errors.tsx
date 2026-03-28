import type { ReactElement } from 'react';
import { useEffect, useMemo, useState } from 'react';

import {
  boundUntrustedText,
  ErrorSummaryList,
  type ErrorSummaryItem,
  ManifestViewer,
  sanitizeUntrustedText,
} from '../../../../packages/ui';

import {
  coerceBridgeError,
  invoke,
  parseBridgeError,
  type BridgeErrorPayload,
} from '../lib/transport';
import { controlPlaneStore, type ControlPlaneState } from '../hooks/use-control-plane';

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
      agentSessionId: 'session-live-1',
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
      agentSessionId: 'session-live-2',
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
      agentSessionId: 'session-hostile-1',
      resolved: false,
    },
  ],
  severityMatrix: [
    {
      errorManifestId: 'err-severity-1',
      processId: 'proc-100',
      pipelineStage: 'Enrich',
      errorType: 'AGENT_DAEMON_CRASH_DURING_SESSION',
      errorMessage: 'Session daemon crashed before artifact sync.',
      cleanupVerdict: 'leaked',
      severity: 'critical',
      createdAt: '2026-03-28T06:45:00Z',
      agentSessionId: 'session-critical-1',
      resolved: false,
    },
    {
      errorManifestId: 'err-severity-2',
      processId: 'proc-101',
      pipelineStage: 'Qualify',
      errorType: 'SOURCE_CHANGED',
      errorMessage: 'Notice source changed during qualification.',
      cleanupVerdict: 'clean',
      severity: 'warning',
      createdAt: '2026-03-28T06:50:00Z',
      agentSessionId: 'session-warning-1',
      resolved: false,
    },
    {
      errorManifestId: 'err-severity-3',
      processId: 'proc-102',
      pipelineStage: 'Extract',
      errorType: 'PIPELINE_QUALITY_GATE_FAILED',
      errorMessage: 'Quality gate rejected the extracted packet.',
      cleanupVerdict: 'clean',
      severity: 'warning',
      createdAt: '2026-03-28T06:55:00Z',
      agentSessionId: 'session-warning-2',
      resolved: false,
    },
  ],
};

function currentFixtureKey(): string | null {
  return new URLSearchParams(globalThis.location?.search ?? '').get('errorsFixture');
}

function currentFixture(): ErrorSummaryItem[] {
  const fixture = currentFixtureKey();
  return fixture ? fixtureErrors[fixture] ?? fixtureErrors.default : fixtureErrors.default;
}

function unresolvedOnly(items: ErrorSummaryItem[]): ErrorSummaryItem[] {
  return items.filter((item) => !item.resolved);
}

function humanizeControl(control: ControlPlaneState['control']): string {
  return control.replace('_', ' ');
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
    `- Session: ${error.agentSessionId ?? 'No session linked'}`,
    `- Severity: ${error.severity}`,
    `- Cleanup: ${error.cleanupVerdict ?? 'n/a'}`,
  ].join('\n');
}

function describeMutationFailure(
  command: 'resolve_error' | 'rerun_from_error',
  error: BridgeErrorPayload,
): string {
  const action = command === 'rerun_from_error' ? 'Rerun' : 'Resolve';
  switch (error.code) {
    case 'transport_unavailable':
      return `${action} failed because the daemon bridge is unavailable. Desktop switched to browse-only mode.`;
    case 'conflict':
      return `${action} blocked by conflict: ${error.message}`;
    case 'invalid_params':
      return `${action} rejected (invalid_params): ${error.message}`;
    case 'busy':
      return `${action} delayed while the daemon is busy: ${error.message}`;
    default:
      return `${action} failed (${error.code}): ${error.message}`;
  }
}

function degradeToBrowseOnly(message: string): void {
  const current = controlPlaneStore.getSnapshot().state;
  controlPlaneStore.setState({
    ...current,
    daemon: 'unavailable',
    control: 'read_only',
    reasons: [message],
  });
}

export function ErrorsPage({
  controlPlane,
}: {
  controlPlane: ControlPlaneState;
}): ReactElement {
  const fixtureKey = currentFixtureKey();
  const [errors, setErrors] = useState<ErrorSummaryItem[]>(
    fixtureKey ? unresolvedOnly(currentFixture()) : [],
  );
  const [selectedId, setSelectedId] = useState<string | null>(
    fixtureKey ? unresolvedOnly(currentFixture())[0]?.errorManifestId ?? null : null,
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
  const [queryMessage, setQueryMessage] = useState<string | null>(null);
  const [retryableMutation, setRetryableMutation] = useState<{
    command: 'resolve_error' | 'rerun_from_error';
    errorManifestId: string;
  } | null>(null);
  const [lastMutationKind, setLastMutationKind] = useState<
    'resolve_error' | 'rerun_from_error' | null
  >(null);

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
        const unresolved = unresolvedOnly(envelope.data.items);
        setErrors(unresolved);
        setSelectedId(unresolved[0]?.errorManifestId ?? null);
        setQueryMessage(null);
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
    setRetryableMutation(null);
    setLastMutationKind(command);
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
      const bridgeError = parseBridgeError(result);
      if (bridgeError) {
        throw bridgeError;
      }
      throw new Error(`${command} failed`);
    } catch (error) {
      const bridgeError = coerceBridgeError(error, `${command} failed`);
      if (bridgeError.code === 'transport_unavailable') {
        degradeToBrowseOnly(
          `Daemon bridge unavailable during ${command.replaceAll('_', ' ')}. ${bridgeError.message}`,
        );
      }
      if (bridgeError.retryable) {
        setRetryableMutation({ command, errorManifestId });
      }
      setMutationMessage(describeMutationFailure(command, bridgeError));
    } finally {
      setMutationPending(false);
    }
  }

  useEffect(() => {
    let cancelled = false;
    if (fixtureKey) {
      const fixture = unresolvedOnly(currentFixture());
      setErrors(fixture);
      setSelectedId(fixture[0]?.errorManifestId ?? null);
      setSnapshotMeta({
        snapshotSeq: 0,
        snapshotAt: '',
        source: `fixture:${fixtureKey}`,
      });
      return () => {
        cancelled = true;
      };
    }

    void invoke('list_errors', {
      cursor: null,
      limit: 25,
      filters: { resolved: false },
    })
      .then((result) => {
        if (cancelled) {
          return;
        }
        const bridgeError = parseBridgeError(result);
        if (bridgeError) {
          if (bridgeError.code === 'transport_unavailable') {
            degradeToBrowseOnly(
              `Daemon bridge unavailable while loading errors. ${bridgeError.message}`,
            );
          }
          setErrors(currentFixture());
          setSelectedId(currentFixture()[0]?.errorManifestId ?? null);
          setQueryMessage(
            `Live error queue unavailable (${bridgeError.code}). Showing local fixture data.`,
          );
          return;
        }
        if (
          result &&
          typeof result === 'object' &&
          'data' in result &&
          'snapshotSeq' in result
        ) {
          const envelope = result as ErrorEnvelope;
          const unresolved = unresolvedOnly(envelope.data.items);
          setErrors(unresolved);
          setSelectedId(unresolved[0]?.errorManifestId ?? null);
          setSnapshotMeta({
            snapshotSeq: envelope.snapshotSeq,
            snapshotAt: envelope.snapshotAt,
            source: 'query',
          });
          return;
        }

        const fallback = unresolvedOnly(currentFixture());
        setErrors(fallback);
        setSelectedId(fallback[0]?.errorManifestId ?? null);
        setQueryMessage('Live error queue unavailable. Showing local fixture data.');
      })
      .catch((error) => {
        if (!cancelled) {
          const bridgeError = coerceBridgeError(error, 'list_errors failed');
          if (bridgeError.code === 'transport_unavailable') {
            degradeToBrowseOnly(
              `Daemon bridge unavailable while loading errors. ${bridgeError.message}`,
            );
          }
          const fallback = unresolvedOnly(currentFixture());
          setErrors(fallback);
          setSelectedId(fallback[0]?.errorManifestId ?? null);
          setQueryMessage(
            `Live error queue unavailable (${bridgeError.code}). Showing local fixture data.`,
          );
        }
      });

    return () => {
      cancelled = true;
    };
  }, [fixtureKey]);

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
            Unresolved manifests remain readable in {humanizeControl(controlPlane.control)} mode.
          </p>
        </div>
        <div className="meta-card">
          <span>Snapshot source</span>
          <strong>{snapshotMeta.source}</strong>
          <span>Seq {snapshotMeta.snapshotSeq}</span>
        </div>
      </header>

      {queryMessage ? <p role="status">{queryMessage}</p> : null}
      {mutationMessage ? <p role="status">{mutationMessage}</p> : null}
      {retryableMutation ? (
        <button
          onClick={() => {
            void runErrorMutation(
              retryableMutation.command,
              retryableMutation.errorManifestId,
            );
          }}
          type="button"
        >
          Retry {retryableMutation.command === 'rerun_from_error' ? 'rerun' : 'resolve'}
        </button>
      ) : null}

      {errors.length === 0 ? (
        <div className="empty-state" role="status">
          {lastMutationKind === 'resolve_error' && !mutationMessage ? (
            <p>Error resolution committed.</p>
          ) : null}
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
              {(() => {
                const rawContext = boundUntrustedText(
                  sanitizeUntrustedText(JSON.stringify(selectedError, null, 2)),
                  4_000,
                );
                return (
                  <>
                    <pre>{rawContext.safeText}</pre>
                    {rawContext.truncated ? (
                      <p role="status">Raw error context truncated for safe rendering.</p>
                    ) : null}
                  </>
                );
              })()}
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
            </aside>
          ) : null}
        </div>
      )}
    </section>
  );
}
