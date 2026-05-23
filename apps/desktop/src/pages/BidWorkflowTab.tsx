import type { ReactElement } from 'react';

import {
  BidScopeSummary,
  BidWorkflowStartButton,
  BidWorkflowStatusBadge,
  DraftSavedReviewPanel,
  type BidWorkflowReadiness,
  type BidWorkflowState,
} from '../../../../packages/ui';
import type {
  BidScopePayload,
  SideEffectClass,
} from '../../../../packages/bidding/bid-scope-types';

export interface DesktopBidWorkflowState {
  bidState: BidWorkflowState;
  updatedAt?: string | null;
  bidScope?: BidScopePayload | null;
  draftRef?: string | null;
  screenshotPaths?: readonly string[];
  manifestHref?: string | null;
  savedAt?: string | null;
  sideEffectCap?: SideEffectClass | null;
}

export interface BidWorkflowTabProps {
  processId: string;
  workflowState?: DesktopBidWorkflowState | null;
  readiness?: BidWorkflowReadiness | null;
  mutationsEnabled: boolean;
  onStartBid?: () => void;
}

function guardedReadiness(
  readiness: BidWorkflowReadiness | null | undefined,
  mutationsEnabled: boolean,
  hasStartHandler: boolean,
): BidWorkflowReadiness {
  if (!readiness) {
    return {
      eligible: false,
      blockers: ['bid_readiness_not_loaded'],
    };
  }

  if (!mutationsEnabled) {
    return {
      ...readiness,
      eligible: false,
      blockers: [...(readiness.blockers ?? []), 'desktop_control_not_available'],
    };
  }

  if (!hasStartHandler) {
    return {
      ...readiness,
      eligible: false,
      blockers: [...(readiness.blockers ?? []), 'bid_scope_not_ready'],
    };
  }

  return readiness;
}

export function BidWorkflowTab({
  processId,
  workflowState,
  readiness,
  mutationsEnabled,
  onStartBid,
}: BidWorkflowTabProps): ReactElement {
  const bidState = workflowState?.bidState ?? 'not_started';
  const startReadiness = guardedReadiness(readiness, mutationsEnabled, Boolean(onStartBid));
  const canShowStart = bidState === 'not_started' || bidState === 'ready_to_bid';

  return (
    <section className="bid-workflow-tab" data-process-id={processId}>
      <header>
        <p className="page__eyebrow">Bid workflow</p>
        <h3>Procontract response</h3>
        <BidWorkflowStatusBadge state={bidState} updatedAt={workflowState?.updatedAt ?? undefined} />
      </header>

      {workflowState?.bidScope ? (
        <BidScopeSummary scope={workflowState.bidScope} viewerMode={false} />
      ) : (
        <p className="empty-state">
          No bid scope has been prepared for this process yet.
        </p>
      )}

      {canShowStart ? (
        <BidWorkflowStartButton
          readiness={startReadiness}
          onStart={() => {
            onStartBid?.();
          }}
        />
      ) : null}

      {bidState === 'draft_saved' && workflowState?.draftRef ? (
        <DraftSavedReviewPanel
          processId={processId}
          draftRef={workflowState.draftRef}
          screenshotPaths={workflowState.screenshotPaths ?? []}
          manifestHref={workflowState.manifestHref ?? undefined}
          savedAt={workflowState.savedAt ?? undefined}
          sideEffectCap={workflowState.sideEffectCap ?? workflowState.bidScope?.sideEffectCap ?? 'REMOTE_DRAFT'}
          submitDisabledReason="Final submission controls are enabled by the review-flow bead."
        />
      ) : null}
    </section>
  );
}
