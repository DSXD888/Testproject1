import { useState, type ReactElement } from 'react';

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
import {
  invoke,
  triggerBidWorkflow,
  type BidDocumentPayload,
  type CompanyProfilePayload,
  type TriggerBidWorkflowParams,
} from '../lib/transport';

type StartStatus = 'idle' | 'starting' | 'queued' | 'error';

type BidReadinessBlocker =
  | string
  | {
      code: string;
      diagnostic?: string | null;
    };

export interface DesktopBidWorkflowReadiness
  extends Omit<BidWorkflowReadiness, 'blockers'> {
  blockers?: readonly BidReadinessBlocker[];
}

export interface DesktopBidWorkflowState {
  bidState: BidWorkflowState;
  updatedAt?: string | null;
  bidScope?: BidScopePayload | null;
  readiness?: DesktopBidWorkflowReadiness | null;
  operatorId?: string | null;
  companyProfile?: CompanyProfilePayload | null;
  bidDocuments?: readonly BidDocumentPayload[];
  stageRunId?: string | null;
  draftRef?: string | null;
  screenshotPaths?: readonly string[];
  manifestHref?: string | null;
  savedAt?: string | null;
  sideEffectCap?: SideEffectClass | null;
}

export interface BidWorkflowTabProps {
  processId: string;
  workflowState?: DesktopBidWorkflowState | null;
  readiness?: DesktopBidWorkflowReadiness | null;
  mutationsEnabled: boolean;
  onStartBid?: () => void | Promise<void>;
}

function blockerCode(blocker: BidReadinessBlocker): string {
  return typeof blocker === 'string' ? blocker : blocker.code;
}

function blockerDiagnostic(blocker: BidReadinessBlocker): string | null {
  return typeof blocker === 'string' ? null : (blocker.diagnostic ?? null);
}

function appendBlocker(
  readiness: DesktopBidWorkflowReadiness,
  blocker: BidReadinessBlocker,
): DesktopBidWorkflowReadiness {
  return {
    ...readiness,
    blockers: [...(readiness.blockers ?? []), blocker],
  };
}

function buttonReadiness(
  readiness: DesktopBidWorkflowReadiness,
): BidWorkflowReadiness {
  return {
    ...readiness,
    blockers: readiness.blockers?.map(blockerCode),
  };
}

function guardedReadiness(
  readiness: DesktopBidWorkflowReadiness | null | undefined,
  mutationsEnabled: boolean,
  hasStartHandler: boolean,
): DesktopBidWorkflowReadiness {
  if (!readiness) {
    return {
      eligible: false,
      blockers: ['bid_readiness_not_loaded'],
    };
  }

  if (!mutationsEnabled) {
    return appendBlocker(
      {
        ...readiness,
        eligible: false,
      },
      'desktop_control_not_available',
    );
  }

  if (!hasStartHandler) {
    return appendBlocker(
      {
        ...readiness,
        eligible: false,
      },
      'bid_scope_not_ready',
    );
  }

  return readiness;
}

function buildTriggerParams(
  workflowState: DesktopBidWorkflowState | null | undefined,
): TriggerBidWorkflowParams | null {
  if (
    !workflowState?.bidScope ||
    !workflowState.operatorId ||
    !workflowState.companyProfile ||
    !workflowState.stageRunId
  ) {
    return null;
  }

  return {
    processId: workflowState.bidScope.processId,
    bidScope: workflowState.bidScope as TriggerBidWorkflowParams['bidScope'],
    operatorId: workflowState.operatorId,
    companyProfile: workflowState.companyProfile,
    bidDocuments: [...(workflowState.bidDocuments ?? [])],
    stageRunId: workflowState.stageRunId,
  };
}

export function BidWorkflowTab({
  processId,
  workflowState,
  readiness,
  mutationsEnabled,
  onStartBid,
}: BidWorkflowTabProps): ReactElement {
  const [startStatus, setStartStatus] = useState<StartStatus>('idle');
  const [startError, setStartError] = useState<string | null>(null);
  const bidState = workflowState?.bidState ?? 'not_started';
  const triggerParams = buildTriggerParams(workflowState);
  const startReadiness = guardedReadiness(
    readiness ?? workflowState?.readiness,
    mutationsEnabled,
    Boolean(onStartBid) || Boolean(triggerParams),
  );
  const canShowStart = bidState === 'not_started' || bidState === 'ready_to_bid';
  const blockerDetails = startReadiness.blockers ?? [];

  const handleStartBid = async (): Promise<void> => {
    if (!startReadiness.eligible || startStatus === 'starting') {
      return;
    }

    setStartStatus('starting');
    setStartError(null);

    try {
      if (onStartBid) {
        await onStartBid();
      } else if (triggerParams) {
        const response = await triggerBidWorkflow({ invoke }, triggerParams);
        if (!response.accepted) {
          throw new Error('Bid workflow was not accepted by the daemon.');
        }
      } else {
        throw new Error('Bid scope is not ready for submission.');
      }
      setStartStatus('queued');
    } catch (error) {
      setStartStatus('error');
      setStartError(error instanceof Error ? error.message : 'Bid workflow start failed.');
    }
  };

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
          readiness={buttonReadiness(startReadiness)}
          busy={startStatus === 'starting'}
          onStart={() => void handleStartBid()}
        />
      ) : null}

      {!startReadiness.eligible && blockerDetails.length > 0 ? (
        <section data-testid="readiness-blockers" role="status">
          <h4>Readiness blockers</h4>
          <ul>
            {blockerDetails.map((blocker) => (
              <li key={blockerCode(blocker)}>
                <strong>{blockerCode(blocker)}</strong>
                {blockerDiagnostic(blocker) ? (
                  <p>{blockerDiagnostic(blocker)}</p>
                ) : null}
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {startStatus === 'queued' ? (
        <p data-testid="start-bid-status" role="status">
          Bid workflow queued.
        </p>
      ) : null}
      {startStatus === 'error' ? (
        <p data-testid="start-bid-error" role="alert">
          {startError}
        </p>
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
