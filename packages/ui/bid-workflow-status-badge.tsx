import type { ReactElement } from 'react';

export type BidWorkflowState =
  | 'not_started'
  | 'ready_to_bid'
  | 'bid_in_progress'
  | 'draft_saved'
  | 'bid_submitted'
  | 'bid_failed'
  | 'portal_state_unknown';

export interface BidWorkflowStatusBadgeProps {
  state: BidWorkflowState;
  updatedAt?: string;
  size?: 'sm' | 'md' | 'lg';
}

interface BidWorkflowStateMeta {
  icon: string;
  label: string;
  tone: 'muted' | 'info' | 'active' | 'review' | 'good' | 'bad' | 'warn';
}

const BID_WORKFLOW_STATE_META: Record<BidWorkflowState, BidWorkflowStateMeta> = {
  not_started: { icon: 'NS', label: 'Not started', tone: 'muted' },
  ready_to_bid: { icon: 'RD', label: 'Ready to bid', tone: 'info' },
  bid_in_progress: { icon: 'IP', label: 'Bid in progress', tone: 'active' },
  draft_saved: { icon: 'DR', label: 'Draft saved', tone: 'review' },
  bid_submitted: { icon: 'SB', label: 'Bid submitted', tone: 'good' },
  bid_failed: { icon: 'FL', label: 'Bid failed', tone: 'bad' },
  portal_state_unknown: { icon: 'UN', label: 'Portal state unknown', tone: 'warn' },
};

export function BidWorkflowStatusBadge({
  state,
  updatedAt,
  size = 'md',
}: BidWorkflowStatusBadgeProps): ReactElement {
  const meta = BID_WORKFLOW_STATE_META[state];

  return (
    <span
      className="bid-workflow-status-badge"
      data-size={size}
      data-state={state}
      data-tone={meta.tone}
      title={`${meta.label} state`}
    >
      <span aria-hidden="true" className="bid-workflow-status-badge__icon">
        {meta.icon}
      </span>
      <span className="bid-workflow-status-badge__label">{meta.label}</span>
      {updatedAt ? (
        <time className="bid-workflow-status-badge__timestamp" dateTime={updatedAt}>
          {updatedAt}
        </time>
      ) : null}
    </span>
  );
}
