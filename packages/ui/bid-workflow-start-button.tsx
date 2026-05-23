import type { ReactElement } from 'react';

export interface BidWorkflowReadiness {
  eligible: boolean;
  blockers?: readonly string[];
  overrideRequired?: string | null;
  checkedAt?: string;
}

export interface BidWorkflowStartButtonProps {
  readiness: BidWorkflowReadiness;
  onStart: () => void;
  busy?: boolean;
  label?: string;
}

export function BidWorkflowStartButton({
  readiness,
  onStart,
  busy = false,
  label = 'Start bid',
}: BidWorkflowStartButtonProps): ReactElement {
  const blockers = readiness.blockers ?? [];
  const disabled = busy || !readiness.eligible;
  const statusLabel = readiness.eligible ? 'Ready to start' : 'Bid readiness blocked';

  return (
    <div
      className="bid-workflow-start-button"
      data-eligible={readiness.eligible ? 'true' : 'false'}
    >
      <button type="button" disabled={disabled} onClick={onStart}>
        {busy ? 'Starting bid' : label}
      </button>
      <p className="bid-workflow-start-button__status" role="status">
        {statusLabel}
      </p>
      {readiness.checkedAt ? (
        <time dateTime={readiness.checkedAt}>{readiness.checkedAt}</time>
      ) : null}
      {readiness.overrideRequired ? (
        <p className="bid-workflow-start-button__override">
          Override required: {readiness.overrideRequired}
        </p>
      ) : null}
      {blockers.length > 0 ? (
        <ul className="bid-workflow-start-button__blockers">
          {blockers.map((blocker) => (
            <li key={blocker}>{blocker}</li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
