import type { ReactElement } from 'react';

export interface ErrorSummaryItem {
  errorManifestId: string;
  processId: string | null;
  pipelineStage: string | null;
  errorType: string;
  errorMessage: string;
  cleanupVerdict?: 'clean' | 'forced' | 'leaked' | null;
  severity: 'critical' | 'warning' | 'info';
  createdAt: string;
  agentSessionId?: string | null;
  resolved: boolean;
  resolvedAt?: string;
}

export interface ErrorSummaryListProps {
  errors: ErrorSummaryItem[];
  onErrorClick?: (errorManifestId: string) => void;
  onResolveClick?: ((errorManifestId: string) => void) | null;
}

const SEVERITY_LABEL: Record<ErrorSummaryItem['severity'], string> = {
  critical: 'Critical',
  warning: 'Warning',
  info: 'Info',
};

const SEVERITY_ICON: Record<ErrorSummaryItem['severity'], string> = {
  critical: 'CR',
  warning: 'WR',
  info: 'IN',
};

export function ErrorSummaryList({
  errors,
  onErrorClick,
  onResolveClick,
}: ErrorSummaryListProps): ReactElement {
  return (
    <ul className="error-summary-list">
      {errors.map((error) => (
        <li
          key={error.errorManifestId}
          className="error-summary-list__item"
          data-severity={error.severity}
          data-resolved={error.resolved ? 'true' : 'false'}
        >
          <div className="error-summary-list__header">
            <span aria-hidden="true">{SEVERITY_ICON[error.severity]}</span>
            <strong>{SEVERITY_LABEL[error.severity]}</strong>
            <span>{error.errorType}</span>
          </div>

          <p>{error.errorMessage}</p>
          <dl>
            <div>
              <dt>Process</dt>
              <dd>{error.processId ?? 'No process linked'}</dd>
            </div>
            <div>
              <dt>Stage</dt>
              <dd>{error.pipelineStage ?? 'Unscoped'}</dd>
            </div>
            <div>
              <dt>Cleanup</dt>
              <dd>{error.cleanupVerdict ?? 'n/a'}</dd>
            </div>
            <div>
              <dt>Created</dt>
              <dd>{error.createdAt}</dd>
            </div>
            {error.agentSessionId ? (
              <div>
                <dt>Session</dt>
                <dd>{error.agentSessionId}</dd>
              </div>
            ) : null}
            <div>
              <dt>Status</dt>
              <dd>{error.resolved ? `Resolved ${error.resolvedAt ?? ''}`.trim() : 'Open'}</dd>
            </div>
          </dl>

          <div className="error-summary-list__actions">
            {onErrorClick ? (
              <button type="button" onClick={() => onErrorClick(error.errorManifestId)}>
                View details
              </button>
            ) : null}
            {onResolveClick ? (
              <button type="button" onClick={() => onResolveClick(error.errorManifestId)}>
                Resolve
              </button>
            ) : null}
          </div>
        </li>
      ))}
    </ul>
  );
}
