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

type FormalSubmissionAuthorization = {
  reason: string;
  recorded_by: string;
  recorded_at: string;
};

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
  agentSessionId?: string | null;
  formalSubmissionAuthorization?: FormalSubmissionAuthorization | null;
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

function withSideEffectCap(
  params: TriggerBidWorkflowParams | null,
  sideEffectCap: SideEffectClass,
): TriggerBidWorkflowParams | null {
  if (!params) {
    return null;
  }
  return {
    ...params,
    bidScope: {
      ...params.bidScope,
      sideEffectCap,
    },
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
  return Object.is(verdict, 'rejected') || Object.is(verdict, 'outage');
}

function isObjectRecord(value: unknown): value is Record<string, unknown> {
  if (!value) {
    return false;
  }
  return Object.is(Object(value), value);
}

function isAcceptedMutation(value: unknown): boolean {
  if (!isObjectRecord(value)) {
    return false;
  }
  return Object.is(value.accepted, true);
}

function isBidState(state: BidWorkflowState, expected: BidWorkflowState): boolean {
  return Object.is(state, expected);
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
  const [capAuthorizationFormOpen, setCapAuthorizationFormOpen] = useState(false);
  const [capAuthorizationReason, setCapAuthorizationReason] = useState('');
  const [capAuthorizationConfirmed, setCapAuthorizationConfirmed] = useState(false);
  const [capAuthorizationError, setCapAuthorizationError] = useState<string | null>(null);
  const [localFormalAuthorization, setLocalFormalAuthorization] =
    useState<FormalSubmissionAuthorization | null>(null);
  const bidState = workflowState?.bidState ?? 'not_started';
  const triggerParams = buildTriggerParams(workflowState);
  const startReadiness = guardedReadiness(
    readiness ?? workflowState?.readiness,
    mutationsEnabled,
    Boolean(onStartBid) || Boolean(triggerParams),
  );
  const canShowStart = isBidState(bidState, 'not_started') || isBidState(bidState, 'ready_to_bid');
  const blockerDetails = startReadiness.blockers ?? [];
  const operatorId = workflowState?.operatorId ?? 'operator-unknown';
  const bidSideEffectCap =
    workflowState?.sideEffectCap ?? workflowState?.bidScope?.sideEffectCap ?? 'REMOTE_DRAFT';
  const formalSubmissionAuthorization =
    workflowState?.formalSubmissionAuthorization ?? localFormalAuthorization;
  const effectiveSideEffectCap: SideEffectClass =
    formalSubmissionAuthorization && isBidState(bidState, 'draft_saved')
      ? 'FORMAL_SUBMISSION'
      : bidSideEffectCap;
  const effectiveTriggerParams = withSideEffectCap(
    triggerParams,
    effectiveSideEffectCap,
  );
  const quoteRoleOverrideAvailable = blockerDetails.some(
    (blocker) => blockerCode(blocker) === 'QUOTE_ROLE_FLOOR_ONLY',
  );
  const validationOverrideAvailable =
    isBidState(bidState, 'draft_saved') &&
    canUseValidationOverride(workflowState?.validatorVerdict);
  const finalSubmissionAuthorized =
    sideEffectAllowsFormalSubmission(effectiveSideEffectCap) &&
    Boolean(formalSubmissionAuthorization);
  const validationOverrideCanSubmit =
    validationOverrideAvailable && finalSubmissionAuthorized;
  const startStatusMessage = renderStartStatusMessage(startStatus, startError);

  const handleStartBid = async (): Promise<void> => {
    if (!startReadiness.eligible || Object.is(startStatus, 'starting')) {
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
      sideEffectCap: effectiveSideEffectCap,
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
    if (!effectiveTriggerParams) {
      throw new Error('Bid scope is not ready for final submission.');
    }

    await recordOverride(record);
    const response = await triggerBidWorkflow({ invoke }, effectiveTriggerParams);
    if (!response.accepted) {
      throw new Error('Final bid submission workflow was not accepted.');
    }
  };

  const finalSubmitDisabledReason = (() => {
    if (!sideEffectAllowsFormalSubmission(effectiveSideEffectCap)) {
      return 'Final submission requires sideEffectCap FORMAL_SUBMISSION.';
    }
    if (!formalSubmissionAuthorization) {
      return 'Final submission requires recorded operator authorization.';
    }
    if (workflowState?.validatorVerdict !== 'approved') {
      return 'Final submission requires validator approval or validation override before portal submission.';
    }
    if (!effectiveTriggerParams) {
      return 'Bid scope is not ready for final submission.';
    }
    return undefined;
  })();

  const handleSubmitFinal = async (): Promise<void> => {
    if (finalSubmitDisabledReason || !effectiveTriggerParams) {
      return;
    }

    setStartStatus('starting');
    setStartError(null);
    try {
      const response = await triggerBidWorkflow({ invoke }, effectiveTriggerParams);
      if (!response.accepted) {
        throw new Error('Final bid submission workflow was not accepted.');
      }
      setStartStatus('queued');
    } catch (error) {
      setStartStatus('error');
      setStartError(
        error instanceof Error ? error.message : 'Final bid submission failed.',
      );
    }
  };

  const handleFormalAuthorization = (): void => {
    const trimmedReason = capAuthorizationReason.trim();
    if (!trimmedReason) {
      setCapAuthorizationError('Reason is required.');
      return;
    }
    if (!capAuthorizationConfirmed) {
      setCapAuthorizationError('Operator confirmation is required.');
      return;
    }

    setLocalFormalAuthorization({
      reason: trimmedReason,
      recorded_by: operatorId,
      recorded_at: new Date().toISOString(),
    });
    setCapAuthorizationError(null);
    setCapAuthorizationFormOpen(false);
  };

  const openBidSessionViewer = (sessionId: string): void => {
    window.history.pushState(
      {},
      '',
      `/sessions/${encodeURIComponent(sessionId)}?initialMode=history`,
    );
    window.dispatchEvent(new PopStateEvent('popstate'));
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
        <>
          <DraftSavedReviewPanel
            processId={processId}
            draftRef={workflowState.draftRef}
            screenshotPaths={workflowState.screenshotPaths ?? []}
            manifestHref={workflowState.manifestHref ?? undefined}
            savedAt={workflowState.savedAt ?? undefined}
            sideEffectCap={effectiveSideEffectCap}
            onSubmitFinal={
              finalSubmitDisabledReason ? undefined : () => void handleSubmitFinal()
            }
            onOpenSessionViewer={
              workflowState.agentSessionId
                ? () => openBidSessionViewer(workflowState.agentSessionId ?? '')
                : undefined
            }
            submitDisabledReason={finalSubmitDisabledReason}
          />
          <section
            className="bid-workflow-cap-authorization"
            data-testid="formal-submission-authorization"
          >
            <button
              type="button"
              data-testid="raise-cap-to-formal-submission"
              onClick={() => setCapAuthorizationFormOpen(true)}
            >
              Authorize formal submission
            </button>
            {formalSubmissionAuthorization ? (
              <p role="status" data-testid="formal-submission-authorization-status">
                Formal submission authorized by{' '}
                {formalSubmissionAuthorization.recorded_by}.
              </p>
            ) : null}
            {capAuthorizationFormOpen ? (
              <form
                data-testid="formal-submission-authorization-form"
                onSubmit={(event) => {
                  event.preventDefault();
                  handleFormalAuthorization();
                }}
              >
                <label>
                  Authorization reason
                  <textarea
                    data-testid="auth-reason"
                    value={capAuthorizationReason}
                    onChange={(event) => setCapAuthorizationReason(event.target.value)}
                    rows={3}
                  />
                </label>
                <label>
                  <input
                    data-testid="auth-confirmation"
                    type="checkbox"
                    checked={capAuthorizationConfirmed}
                    onChange={(event) =>
                      setCapAuthorizationConfirmed(event.target.checked)
                    }
                  />
                  I confirm this operator authorization raises the side-effect
                  cap to FORMAL_SUBMISSION.
                </label>
                <button type="submit" data-testid="auth-confirm">
                  Confirm authorization
                </button>
                {capAuthorizationError ? (
                  <p role="alert" data-testid="auth-error">
                    {capAuthorizationError}
                  </p>
                ) : null}
              </form>
            ) : null}
          </section>
        </>
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
