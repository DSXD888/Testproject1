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
import {
  QuoteRoleOverrideForm,
  type BidWorkflowOverrideRecord,
} from './QuoteRoleOverrideForm';
import { ValidationOverrideForm } from './ValidationOverrideForm';

type StartStatus = 'idle' | 'starting' | 'queued' | 'error';
type OverrideFormKind = 'quoteRole' | 'validation';
type BidWorkflowValidatorVerdict = 'approved' | 'rejected' | 'outage' | 'missing';

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
  validatorVerdict?: BidWorkflowValidatorVerdict | null;
  validatorMessage?: string | null;
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

const SIDE_EFFECT_CLASS_ORDER: readonly SideEffectClass[] = [
  'READ_ONLY',
  'REMOTE_DRAFT',
  'NOTIFICATION_OR_LOCK',
  'LEGAL_DECLARATION',
  'FORMAL_SUBMISSION',
  'FUNDS_COMMITTED',
];

function sideEffectAllowsFormalSubmission(
  cap: SideEffectClass | null | undefined,
): boolean {
  return (
    SIDE_EFFECT_CLASS_ORDER.indexOf(cap ?? 'READ_ONLY') >=
    SIDE_EFFECT_CLASS_ORDER.indexOf('FORMAL_SUBMISSION')
  );
}

function canUseValidationOverride(
  verdict: BidWorkflowValidatorVerdict | null | undefined,
): boolean {
  return verdict === 'rejected' || verdict === 'outage';
}

function isAcceptedMutation(value: unknown): boolean {
  if (!value || typeof value !== 'object') {
    return false;
  }
  const candidate = value as { accepted?: unknown };
  return candidate.accepted === true;
}

function renderStartStatusMessage(
  status: StartStatus,
  error: string | null,
): ReactElement | null {
  switch (status) {
    case 'queued':
      return (
        <p data-testid="start-bid-status" role="status">
          Bid workflow queued.
        </p>
      );
    case 'error':
      return (
        <p data-testid="start-bid-error" role="alert">
          {error}
        </p>
      );
    case 'idle':
    case 'starting':
      return null;
  }
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
  const [activeOverrideForm, setActiveOverrideForm] =
    useState<OverrideFormKind | null>(null);
  const bidState = workflowState?.bidState ?? 'not_started';
  const triggerParams = buildTriggerParams(workflowState);
  const startReadiness = guardedReadiness(
    readiness ?? workflowState?.readiness,
    mutationsEnabled,
    Boolean(onStartBid) || Boolean(triggerParams),
  );
  const canShowStart = bidState === 'not_started' || bidState === 'ready_to_bid';
  const blockerDetails = startReadiness.blockers ?? [];
  const operatorId = workflowState?.operatorId ?? 'operator-unknown';
  const bidSideEffectCap =
    workflowState?.sideEffectCap ?? workflowState?.bidScope?.sideEffectCap ?? 'REMOTE_DRAFT';
  const quoteRoleOverrideAvailable = blockerDetails.some(
    (blocker) => blockerCode(blocker) === 'QUOTE_ROLE_FLOOR_ONLY',
  );
  const validationOverrideAvailable =
    bidState === 'draft_saved' &&
    canUseValidationOverride(workflowState?.validatorVerdict);
  const validationOverrideCanSubmit =
    validationOverrideAvailable && sideEffectAllowsFormalSubmission(bidSideEffectCap);
  const startStatusMessage = renderStartStatusMessage(startStatus, startError);

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

  const recordOverride = async (
    record: BidWorkflowOverrideRecord,
  ): Promise<void> => {
    const response = await invoke('record_bid_workflow_override', {
      processId,
      operatorOverride: record,
      decisionBasisHash: workflowState?.bidScope?.decisionBasisHash ?? null,
      stageRunId: workflowState?.stageRunId ?? null,
      sideEffectCap: bidSideEffectCap,
    });

    if (!isAcceptedMutation(response)) {
      throw new Error('Bid workflow override was not accepted by the daemon.');
    }
  };

  const handleQuoteRoleOverride = async (
    record: BidWorkflowOverrideRecord,
  ): Promise<void> => {
    await recordOverride(record);
  };

  const handleValidationOverride = async (
    record: BidWorkflowOverrideRecord,
  ): Promise<void> => {
    if (!validationOverrideCanSubmit) {
      throw new Error('Formal submission is not authorized for this bid scope.');
    }
    if (!triggerParams) {
      throw new Error('Bid scope is not ready for final submission.');
    }

    await recordOverride(record);
    const response = await triggerBidWorkflow({ invoke }, triggerParams);
    if (!response.accepted) {
      throw new Error('Final bid submission workflow was not accepted.');
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

      {quoteRoleOverrideAvailable ? (
        <section className="bid-workflow-override" data-testid="quote-role-override">
          <button
            type="button"
            data-testid="override-quote-role"
            onClick={() => setActiveOverrideForm('quoteRole')}
          >
            Override and acknowledge
          </button>
          {activeOverrideForm === 'quoteRole' ? (
            <QuoteRoleOverrideForm
              processId={processId}
              recordedBy={operatorId}
              onSubmit={handleQuoteRoleOverride}
            />
          ) : null}
        </section>
      ) : null}

      {startStatusMessage}

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

      {validationOverrideAvailable ? (
        <section className="bid-workflow-override" data-testid="validation-override">
          <button
            type="button"
            data-testid="override-validation"
            disabled={!validationOverrideCanSubmit}
            onClick={() => setActiveOverrideForm('validation')}
          >
            Override and submit final
          </button>
          {!validationOverrideCanSubmit ? (
            <p role="status" data-testid="validation-override-blocker">
              Formal submission requires sideEffectCap FORMAL_SUBMISSION and
              recorded operator authorization.
            </p>
          ) : null}
          {workflowState?.validatorMessage ? (
            <p>{workflowState.validatorMessage}</p>
          ) : null}
          {activeOverrideForm === 'validation' ? (
            <ValidationOverrideForm
              processId={processId}
              recordedBy={operatorId}
              onSubmit={handleValidationOverride}
            />
          ) : null}
        </section>
      ) : null}
    </section>
  );
}
