import { useEffect, useMemo, useState } from 'react';
import type { ReactElement } from 'react';

import { invoke, parseBridgeError } from '../lib/transport';

export type DecisionStatusVm = {
  disposition: 'approve' | 'reject' | 'watch' | null;
  isStale: boolean;
  decidedAt: string | null;
};

export type ManifestInboxRow = {
  processId: string;
  canonicalProcessId: string;
  title: string | null;
  buyerName: string | null;
  estimatedValuePence: number | null;
  stoplight: string | null;
  suggestedAction: string | null;
  clarificationPromptCount: number;
  manifestBasisHash: string | null;
  generatedAt: string;
  decision: DecisionStatusVm;
};

type QueryEnvelope<T> = {
  data: T;
  snapshotSeq: number;
  snapshotAt: string;
};

type InboxState =
  | { status: 'idle' | 'loading' }
  | { status: 'error'; message: string }
  | { status: 'ready'; rows: ManifestInboxRow[]; snapshotSeq: number; snapshotAt: string };

const STOPLIGHT_TONES: Record<string, { background: string; color: string; label: string }> = {
  pursue: { background: '#1f8a4f', color: '#fff', label: 'Pursue' },
  pass: { background: '#c0392b', color: '#fff', label: 'Pass' },
  clarify: { background: '#e0a200', color: '#1a1a1a', label: 'Clarify' },
  review: { background: '#5d6d7e', color: '#fff', label: 'Review' },
};

function formatPence(value: number | null): string {
  if (value === null) return 'Value unknown';
  return new Intl.NumberFormat('en-GB', {
    style: 'currency',
    currency: 'GBP',
    maximumFractionDigits: 0,
  }).format(value / 100);
}

function formatTimestamp(value: string): string {
  try {
    return new Intl.DateTimeFormat('en-GB', {
      day: '2-digit',
      month: 'short',
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(value));
  } catch {
    return value;
  }
}

function StoplightBadge({ value }: { value: string | null }): ReactElement {
  const matched = value ? STOPLIGHT_TONES[value] : undefined;
  const tone = matched ?? {
    background: '#888',
    color: '#fff',
    label: value ?? 'Unknown',
  };
  return (
    <span
      data-testid="stoplight-badge"
      data-stoplight={value ?? 'unknown'}
      style={{
        background: tone.background,
        color: tone.color,
        padding: '2px 8px',
        borderRadius: '4px',
        fontSize: '0.85em',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: '0.05em',
      }}
    >
      {tone.label}
    </span>
  );
}

function DecisionPill({ decision }: { decision: DecisionStatusVm }): ReactElement {
  if (!decision.disposition) {
    return (
      <span data-testid="decision-pill" data-disposition="none" style={pillStyle('#666')}>
        No decision
      </span>
    );
  }
  const color =
    decision.disposition === 'approve'
      ? '#1f8a4f'
      : decision.disposition === 'reject'
        ? '#c0392b'
        : '#5d6d7e';
  const dispositionLabel = decision.disposition.charAt(0).toUpperCase() + decision.disposition.slice(1);
  const label = decision.isStale ? `${dispositionLabel} (stale)` : dispositionLabel;
  return (
    <span
      data-testid="decision-pill"
      data-disposition={decision.disposition}
      data-stale={decision.isStale}
      style={pillStyle(color)}
    >
      {label}
    </span>
  );
}

function pillStyle(color: string): React.CSSProperties {
  return {
    background: color,
    color: '#fff',
    padding: '2px 8px',
    borderRadius: '4px',
    fontSize: '0.8em',
    fontWeight: 500,
  };
}

export type ManifestInboxProps = {
  onOpenProcess: (processId: string) => void;
  /** Optional injection point for tests — bypasses the real `invoke` call. */
  loader?: () => Promise<QueryEnvelope<ManifestInboxRow[]>>;
  /** Force a refetch when this number changes. */
  refreshKey?: number;
  /** Stage 14 Phase 2 — SSR test seam: pre-populate the inbox so it renders
   * the cards in static markup without running useEffect. */
  seedRows?: ManifestInboxRow[];
};

export function ManifestInbox({
  onOpenProcess,
  loader,
  refreshKey,
  seedRows,
}: ManifestInboxProps): ReactElement {
  const [state, setState] = useState<InboxState>(
    seedRows
      ? { status: 'ready', rows: seedRows, snapshotSeq: 0, snapshotAt: 'seed' }
      : { status: 'idle' },
  );

  useEffect(() => {
    if (seedRows) {
      // Tests pass pre-populated rows; skip the live fetch.
      return;
    }
    let cancelled = false;
    setState({ status: 'loading' });
    const promise = loader
      ? loader()
      : (invoke('list_manifests') as Promise<QueryEnvelope<ManifestInboxRow[]>>);
    promise
      .then((envelope) => {
        if (cancelled) return;
        setState({
          status: 'ready',
          rows: envelope.data,
          snapshotSeq: envelope.snapshotSeq,
          snapshotAt: envelope.snapshotAt,
        });
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        const bridge = parseBridgeError(error);
        setState({
          status: 'error',
          message:
            bridge?.message ??
            (error instanceof Error ? error.message : 'Failed to load manifests'),
        });
      });
    return () => {
      cancelled = true;
    };
  }, [loader, refreshKey, seedRows]);

  const headerNote = useMemo(() => {
    if (state.status !== 'ready') return null;
    return `${state.rows.length} manifest${state.rows.length === 1 ? '' : 's'} ready for review`;
  }, [state]);

  return (
    <section className="page page--manifest-inbox">
      <header className="page__header">
        <div>
          <p className="page__eyebrow">Stage 14 — Operator decision</p>
          <h2>Manifest Inbox</h2>
          <p className="page__lede">
            Each card is a process whose Stage 13 manifest is ready for an operator decision.
            Click into a card to capture clarifications, re-run the pipeline, or capture the
            final disposition.
          </p>
        </div>
        {headerNote ? <p className="page__status">{headerNote}</p> : null}
      </header>

      {state.status === 'idle' || state.status === 'loading' ? (
        <p className="empty-state" data-testid="inbox-loading">
          Loading manifests…
        </p>
      ) : null}

      {state.status === 'error' ? (
        <div className="empty-state" role="alert" data-testid="inbox-error">
          <strong>Unable to load Manifest Inbox.</strong>
          <p>{state.message}</p>
        </div>
      ) : null}

      {state.status === 'ready' && state.rows.length === 0 ? (
        <div className="empty-state" data-testid="inbox-empty">
          <strong>No manifests ready for review.</strong>
          <p>Run the pipeline to Stage 13 first.</p>
        </div>
      ) : null}

      {state.status === 'ready' && state.rows.length > 0 ? (
        <ul className="manifest-inbox" aria-label="Manifest Inbox">
          {state.rows.map((row) => (
            <li key={row.processId} data-testid="manifest-inbox-card">
              <button
                type="button"
                className="manifest-inbox__card"
                onClick={() => onOpenProcess(row.canonicalProcessId)}
                style={{
                  display: 'block',
                  width: '100%',
                  textAlign: 'left',
                  padding: '12px',
                  margin: '8px 0',
                  border: '1px solid #ddd',
                  borderRadius: '6px',
                  background: '#fff',
                  cursor: 'pointer',
                }}
              >
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'baseline',
                    gap: '8px',
                    marginBottom: '6px',
                  }}
                >
                  <strong style={{ fontSize: '1em' }}>
                    {row.title ?? row.processId}
                  </strong>
                  <StoplightBadge value={row.stoplight} />
                </div>
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    fontSize: '0.85em',
                    color: '#555',
                    gap: '12px',
                    flexWrap: 'wrap',
                  }}
                >
                  <span data-testid="card-buyer">{row.buyerName ?? 'Buyer unknown'}</span>
                  <span data-testid="card-value">{formatPence(row.estimatedValuePence)}</span>
                  <span data-testid="card-prompts">
                    {row.clarificationPromptCount} clarification
                    {row.clarificationPromptCount === 1 ? '' : 's'}
                  </span>
                  <span data-testid="card-suggested">
                    {row.suggestedAction ?? 'No suggested action'}
                  </span>
                  <span data-testid="card-updated">
                    Updated {formatTimestamp(row.generatedAt)}
                  </span>
                  <DecisionPill decision={row.decision} />
                </div>
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

export default ManifestInbox;
