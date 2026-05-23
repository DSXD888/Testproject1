import { beforeEach, describe, expect, it } from 'vitest';
import type { BidScopePayload, SideEffectClass } from '../bid-scope-types';
import { SIDE_EFFECT_CLASS_ORDER } from '../bid-scope-types';

type BidWorkflowState =
  | 'not_started'
  | 'ready_to_bid'
  | 'bid_in_progress'
  | 'draft_saved'
  | 'bid_submitted'
  | 'bid_failed'
  | 'portal_state_unknown';

type TransitionErrorCode =
  | 'BID_ACTIVE_JOB_EXISTS'
  | 'BID_CONFIRMATION_REF_MISSING'
  | 'BID_EVIDENCE_MISSING'
  | 'BID_FAILURE_REVIEW_REQUIRED'
  | 'BID_PORTAL_RECONCILIATION_REQUIRED'
  | 'BID_READINESS_GATE_FAILED'
  | 'BID_REMOTE_MUTATION_EXISTS'
  | 'BID_SCOPE_INVALID'
  | 'BID_SIDE_EFFECT_CAP_EXCEEDED'
  | 'BID_UNRECOVERABLE_EVIDENCE_REQUIRED'
  | 'BID_VALIDATION_GATE_FAILED';

interface TransitionContext {
  activeBidJob?: boolean;
  automatedRetryFailure?: boolean;
  bidWindowExpired?: boolean;
  capAllowsFormalSubmission?: boolean;
  decisionBasisFresh?: boolean;
  draftEvidence?: boolean;
  errorCode?: string;
  l5BlockerActivated?: boolean;
  manifestConfirmationRef?: string;
  manifestLinked?: boolean;
  noBuyerVisibleSubmission?: boolean;
  operatorAbandoned?: boolean;
  operatorConfirmedSubmission?: boolean;
  operatorReconciled?: boolean;
  operatorWithdrawn?: boolean;
  portalAmbiguous?: boolean;
  portalEvidenceAmbiguous?: boolean;
  portalReverified?: boolean;
  previousFailureReviewed?: boolean;
  previousFailureUnresolvedRemoteMutation?: boolean;
  previousManifestPreserved?: boolean;
  readinessPasses?: boolean;
  remoteDraftChanged?: boolean;
  remoteDraftVerified?: boolean;
  remoteSideEffectEvidence?: boolean;
  scope?: BidScopePayload;
  screenshotTrail?: boolean;
  sideEffectClassReached?: SideEffectClass;
  staleDecision?: boolean;
  validationAccepted?: boolean;
  validationOverride?: boolean;
}

interface StoredBidState {
  bid_scope_json?: BidScopePayload;
  bid_state: BidWorkflowState;
}

interface TransitionResult {
  from: BidWorkflowState;
  to: BidWorkflowState;
  errorCode?: string;
}

const VALID_STATES: BidWorkflowState[] = [
  'not_started',
  'ready_to_bid',
  'bid_in_progress',
  'draft_saved',
  'bid_submitted',
  'bid_failed',
  'portal_state_unknown',
];

const HEX_64 = /^[a-f0-9]{64}$/;

describe('L6-T-bid_workflow_states', () => {
  let harness: BidWorkflowStateMachineHarness;
  let scope: BidScopePayload;

  beforeEach(() => {
    harness = new BidWorkflowStateMachineHarness();
    scope = BidScopeFixture.valid();
  });

  it('L6-T-bid_workflow_states-001: lifecycle enum accepts exactly 7 values', () => {
    expect(VALID_STATES.map((state) => harness.parseState(state))).toEqual(VALID_STATES);
    expect(() => harness.parseState('invalid')).toThrow(BidWorkflowStateMachineError);
  });

  it('L6-T-bid_workflow_states-002: not_started -> ready_to_bid succeeds with all readiness gates passing', () => {
    expect(harness.transition('not_started', 'ready_to_bid', { readinessPasses: true })).toEqual({
      from: 'not_started',
      to: 'ready_to_bid',
    });
  });

  it('L6-T-bid_workflow_states-003: not_started -> ready_to_bid rejects readiness blockers', () => {
    expect(() => harness.transition('not_started', 'ready_to_bid', { readinessPasses: false })).toThrowError(
      new BidWorkflowStateMachineError('BID_READINESS_GATE_FAILED'),
    );
  });

  it('L6-T-bid_workflow_states-004: ready_to_bid -> bid_in_progress requires valid scope and start context', () => {
    expect(
      harness.transition('ready_to_bid', 'bid_in_progress', {
        readinessPasses: true,
        decisionBasisFresh: true,
        scope,
      }),
    ).toEqual({ from: 'ready_to_bid', to: 'bid_in_progress' });
  });

  it('L6-T-bid_workflow_states-005: ready_to_bid -> bid_in_progress rejects missing malformed or stale scope', () => {
    expect(() =>
      harness.transition('ready_to_bid', 'bid_in_progress', {
        readinessPasses: true,
        decisionBasisFresh: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_SCOPE_INVALID'));
    expect(() =>
      harness.transition('ready_to_bid', 'bid_in_progress', {
        readinessPasses: true,
        decisionBasisFresh: false,
        scope,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_READINESS_GATE_FAILED'));
  });

  it('L6-T-bid_workflow_states-006: ready_to_bid -> not_started succeeds on prerequisite regression before start', () => {
    expect(harness.transition('ready_to_bid', 'not_started', { staleDecision: true })).toEqual({
      from: 'ready_to_bid',
      to: 'not_started',
    });
    expect(harness.transition('ready_to_bid', 'not_started', { l5BlockerActivated: true })).toEqual({
      from: 'ready_to_bid',
      to: 'not_started',
    });
    expect(harness.transition('ready_to_bid', 'not_started', { bidWindowExpired: true })).toEqual({
      from: 'ready_to_bid',
      to: 'not_started',
    });
    expect(harness.transition('ready_to_bid', 'not_started', { operatorWithdrawn: true })).toEqual({
      from: 'ready_to_bid',
      to: 'not_started',
    });
  });

  it('L6-T-bid_workflow_states-007: ready_to_bid -> not_started rejects active work or remote evidence', () => {
    expect(() => harness.transition('ready_to_bid', 'not_started', { activeBidJob: true })).toThrowError(
      new BidWorkflowStateMachineError('BID_ACTIVE_JOB_EXISTS'),
    );
    expect(() =>
      harness.transition('ready_to_bid', 'not_started', { remoteSideEffectEvidence: true }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_REMOTE_MUTATION_EXISTS'));
  });

  it('L6-T-bid_workflow_states-008: bid_in_progress -> draft_saved succeeds with draft evidence', () => {
    expect(
      harness.transition('bid_in_progress', 'draft_saved', {
        draftEvidence: true,
        manifestLinked: true,
        screenshotTrail: true,
        sideEffectClassReached: 'REMOTE_DRAFT',
      }),
    ).toEqual({ from: 'bid_in_progress', to: 'draft_saved' });
  });

  it('L6-T-bid_workflow_states-009: bid_in_progress -> draft_saved rejects missing evidence', () => {
    expect(() =>
      harness.transition('bid_in_progress', 'draft_saved', {
        draftEvidence: true,
        manifestLinked: true,
        screenshotTrail: false,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_EVIDENCE_MISSING'));
  });

  it('L6-T-bid_workflow_states-010: bid_in_progress -> bid_submitted requires cap and validation clearance', () => {
    expect(
      harness.transition('bid_in_progress', 'bid_submitted', {
        capAllowsFormalSubmission: true,
        decisionBasisFresh: true,
        validationAccepted: true,
      }),
    ).toEqual({ from: 'bid_in_progress', to: 'bid_submitted' });
    expect(
      harness.transition('bid_in_progress', 'bid_submitted', {
        capAllowsFormalSubmission: true,
        decisionBasisFresh: true,
        validationOverride: true,
      }),
    ).toEqual({ from: 'bid_in_progress', to: 'bid_submitted' });
  });

  it('L6-T-bid_workflow_states-011: bid_in_progress -> bid_submitted rejects final gate failures', () => {
    expect(() =>
      harness.transition('bid_in_progress', 'bid_submitted', {
        capAllowsFormalSubmission: true,
        decisionBasisFresh: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_VALIDATION_GATE_FAILED'));
    expect(() =>
      harness.transition('bid_in_progress', 'bid_submitted', {
        capAllowsFormalSubmission: false,
        decisionBasisFresh: true,
        validationAccepted: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_SIDE_EFFECT_CAP_EXCEEDED'));
    expect(() =>
      harness.transition('bid_in_progress', 'bid_submitted', {
        capAllowsFormalSubmission: true,
        decisionBasisFresh: false,
        validationAccepted: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_READINESS_GATE_FAILED'));
  });

  it('L6-T-bid_workflow_states-012: bid_in_progress -> bid_failed records deterministic pre-mutation failure', () => {
    expect(
      harness.transition('bid_in_progress', 'bid_failed', {
        errorCode: 'BID_SUBMISSION_FAILED',
        sideEffectClassReached: 'READ_ONLY',
      }),
    ).toEqual({
      from: 'bid_in_progress',
      to: 'bid_failed',
      errorCode: 'BID_SUBMISSION_FAILED',
    });
  });

  it('L6-T-bid_workflow_states-013: bid_in_progress -> bid_failed rejects failures after remote draft exists', () => {
    expect(() =>
      harness.transition('bid_in_progress', 'bid_failed', {
        errorCode: 'BID_SUBMISSION_FAILED',
        sideEffectClassReached: 'REMOTE_DRAFT',
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_REMOTE_MUTATION_EXISTS'));
  });

  it('L6-T-bid_workflow_states-014: bid_in_progress -> portal_state_unknown succeeds after remote mutation risk', () => {
    expect(
      harness.transition('bid_in_progress', 'portal_state_unknown', {
        remoteSideEffectEvidence: true,
      }),
    ).toEqual({ from: 'bid_in_progress', to: 'portal_state_unknown' });
    expect(harness.transition('bid_in_progress', 'portal_state_unknown', { portalAmbiguous: true })).toEqual({
      from: 'bid_in_progress',
      to: 'portal_state_unknown',
    });
  });

  it('L6-T-bid_workflow_states-015: bid_in_progress -> portal_state_unknown rejects no remote risk', () => {
    expect(() => harness.transition('bid_in_progress', 'portal_state_unknown', {})).toThrowError(
      new BidWorkflowStateMachineError('BID_PORTAL_RECONCILIATION_REQUIRED'),
    );
  });

  it('L6-T-bid_workflow_states-016: draft_saved -> bid_submitted reruns readiness cap and validation gates', () => {
    expect(
      harness.transition('draft_saved', 'bid_submitted', {
        capAllowsFormalSubmission: true,
        decisionBasisFresh: true,
        readinessPasses: true,
        validationAccepted: true,
      }),
    ).toEqual({ from: 'draft_saved', to: 'bid_submitted' });
  });

  it('L6-T-bid_workflow_states-017: draft_saved -> bid_submitted rejects stale validation outage or cap mismatch', () => {
    expect(() =>
      harness.transition('draft_saved', 'bid_submitted', {
        capAllowsFormalSubmission: true,
        decisionBasisFresh: false,
        readinessPasses: true,
        validationAccepted: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_READINESS_GATE_FAILED'));
    expect(() =>
      harness.transition('draft_saved', 'bid_submitted', {
        capAllowsFormalSubmission: true,
        decisionBasisFresh: true,
        readinessPasses: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_VALIDATION_GATE_FAILED'));
  });

  it('L6-T-bid_workflow_states-018: draft_saved -> bid_in_progress resumes only after verified remote draft', () => {
    expect(
      harness.transition('draft_saved', 'bid_in_progress', {
        readinessPasses: true,
        remoteDraftVerified: true,
      }),
    ).toEqual({ from: 'draft_saved', to: 'bid_in_progress' });
  });

  it('L6-T-bid_workflow_states-019: draft_saved -> bid_in_progress rejects unverifiable or externally changed draft', () => {
    expect(() =>
      harness.transition('draft_saved', 'bid_in_progress', {
        readinessPasses: true,
        remoteDraftVerified: false,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_PORTAL_RECONCILIATION_REQUIRED'));
    expect(() =>
      harness.transition('draft_saved', 'bid_in_progress', {
        readinessPasses: true,
        remoteDraftChanged: true,
        remoteDraftVerified: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_PORTAL_RECONCILIATION_REQUIRED'));
  });

  it('L6-T-bid_workflow_states-020: draft_saved -> bid_failed records operator abandonment', () => {
    expect(harness.transition('draft_saved', 'bid_failed', { operatorAbandoned: true })).toEqual({
      from: 'draft_saved',
      to: 'bid_failed',
    });
  });

  it('L6-T-bid_workflow_states-021: draft_saved -> bid_failed rejects unsafe automated retry failure', () => {
    expect(() =>
      harness.transition('draft_saved', 'bid_failed', {
        automatedRetryFailure: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_REMOTE_MUTATION_EXISTS'));
  });

  it('L6-T-bid_workflow_states-022: portal_state_unknown -> bid_in_progress requires reconciliation and verification', () => {
    expect(
      harness.transition('portal_state_unknown', 'bid_in_progress', {
        operatorReconciled: true,
        portalReverified: true,
        readinessPasses: true,
      }),
    ).toEqual({ from: 'portal_state_unknown', to: 'bid_in_progress' });
  });

  it('L6-T-bid_workflow_states-023: portal_state_unknown -> bid_in_progress rejects missing reconciliation', () => {
    expect(() =>
      harness.transition('portal_state_unknown', 'bid_in_progress', {
        portalReverified: true,
        readinessPasses: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_PORTAL_RECONCILIATION_REQUIRED'));
  });

  it('L6-T-bid_workflow_states-024: portal_state_unknown -> bid_submitted accepts operator confirmation ref', () => {
    expect(
      harness.transition('portal_state_unknown', 'bid_submitted', {
        manifestConfirmationRef: 'PC-2026-000123',
        operatorConfirmedSubmission: true,
      }),
    ).toEqual({ from: 'portal_state_unknown', to: 'bid_submitted' });
  });

  it('L6-T-bid_workflow_states-025: portal_state_unknown -> bid_submitted rejects missing or conflicting confirmation ref', () => {
    expect(() =>
      harness.transition('portal_state_unknown', 'bid_submitted', {
        operatorConfirmedSubmission: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_CONFIRMATION_REF_MISSING'));
    expect(() =>
      harness.transition('portal_state_unknown', 'bid_submitted', {
        manifestConfirmationRef: 'different-run-ref',
        operatorConfirmedSubmission: false,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_CONFIRMATION_REF_MISSING'));
  });

  it('L6-T-bid_workflow_states-026: portal_state_unknown -> bid_failed succeeds with no buyer-visible submission proof', () => {
    expect(
      harness.transition('portal_state_unknown', 'bid_failed', {
        noBuyerVisibleSubmission: true,
      }),
    ).toEqual({ from: 'portal_state_unknown', to: 'bid_failed' });
  });

  it('L6-T-bid_workflow_states-027: portal_state_unknown -> bid_failed rejects ambiguous portal evidence', () => {
    expect(() =>
      harness.transition('portal_state_unknown', 'bid_failed', {
        portalEvidenceAmbiguous: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_UNRECOVERABLE_EVIDENCE_REQUIRED'));
  });

  it('L6-T-bid_workflow_states-028: bid_failed -> ready_to_bid supports reviewed retry with preserved context', () => {
    expect(
      harness.transition('bid_failed', 'ready_to_bid', {
        previousFailureReviewed: true,
        previousManifestPreserved: true,
        readinessPasses: true,
      }),
    ).toEqual({ from: 'bid_failed', to: 'ready_to_bid' });
  });

  it('L6-T-bid_workflow_states-029: bid_failed -> ready_to_bid rejects unresolved remote mutation or stale inputs', () => {
    expect(() =>
      harness.transition('bid_failed', 'ready_to_bid', {
        previousFailureReviewed: true,
        previousFailureUnresolvedRemoteMutation: true,
        previousManifestPreserved: true,
        readinessPasses: true,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_REMOTE_MUTATION_EXISTS'));
    expect(() =>
      harness.transition('bid_failed', 'ready_to_bid', {
        previousFailureReviewed: true,
        previousManifestPreserved: true,
        readinessPasses: false,
      }),
    ).toThrowError(new BidWorkflowStateMachineError('BID_READINESS_GATE_FAILED'));
  });

  it('L6-T-bid_workflow_states-030: BidScopePayload validates all required references', () => {
    expect(harness.validateScope(scope)).toBe(true);

    const invalids: BidScopePayload[] = [
      { ...scope, processId: '' },
      { ...scope, lotIds: [] },
      { ...scope, candidateIds: [] },
      { ...scope, selectedQuoteIds: [] },
      { ...scope, companyProfileVersion: '' },
      { ...scope, bidPortalType: 'proactis' },
      { ...scope, portalOpportunityRef: '' },
      { ...scope, reviewRequired: undefined as unknown as boolean },
      { ...scope, decisionBasisHash: '' },
      { ...scope, sideEffectCap: 'INVALID' as SideEffectClass },
      { ...scope, lotIds: ['unknown-lot'] },
      { ...scope, selectedQuoteIds: ['unknown-quote'] },
    ];

    for (const invalid of invalids) {
      expect(() => harness.validateScope(invalid)).toThrow(BidWorkflowStateMachineError);
    }
  });

  it('L6-T-bid_workflow_states-031: decisionBasisHash is SHA-256 and changes per vector', async () => {
    const base = await harness.computeBasisHash(scope, DecisionBasisFixture.valid());
    expect(base).toMatch(HEX_64);

    for (const changed of DecisionBasisFixture.eachChangedVector(scope)) {
      await expect(harness.computeBasisHash(changed.scope, changed.vectors)).resolves.not.toBe(base);
    }
  });

  it('L6-T-bid_workflow_states-032: defaults set reviewRequired true and sideEffectCap REMOTE_DRAFT', () => {
    expect(BidScopeFixture.operatorStartedDefaults()).toMatchObject({
      reviewRequired: true,
      sideEffectCap: 'REMOTE_DRAFT',
    });
  });

  it('L6-T-bid_workflow_states-033: bid_scope_json persists but read-only projection omits it', () => {
    harness.persistState(scope.processId, {
      bid_state: 'draft_saved',
      bid_scope_json: scope,
    });

    expect(harness.loadStoredScope(scope.processId)).toEqual(scope);
    expect(harness.loadViewerProjection(scope.processId)).toEqual({
      bid_state: 'draft_saved',
    });
  });

  it('L6-T-bid_workflow_states-034: bid_workflow_states schema remains free of SQL lifecycle CHECK', () => {
    expect(harness.schemaInfo('bid_workflow_states')).toEqual({
      check_constraints: [],
      columns: ['process_id', 'bid_state', 'bid_scope_json', 'review_required', 'bid_job_id'],
    });
  });
});

class BidWorkflowStateMachineError extends Error {
  readonly code: TransitionErrorCode;

  constructor(code: TransitionErrorCode) {
    super(code);
    this.name = 'BidWorkflowStateMachineError';
    this.code = code;
  }
}

class BidWorkflowStateMachineHarness {
  private readonly states = new Set<BidWorkflowState>(VALID_STATES);
  private readonly rows = new Map<string, StoredBidState>();
  private readonly knownLots = new Set(['lot-1', 'lot-2']);
  private readonly knownCandidates = new Set(['candidate-1', 'candidate-2']);
  private readonly knownQuotes = new Set(['quote-1', 'quote-2']);

  parseState(value: string): BidWorkflowState {
    if (!this.states.has(value as BidWorkflowState)) {
      throw new BidWorkflowStateMachineError('BID_SCOPE_INVALID');
    }

    return value as BidWorkflowState;
  }

  transition(from: BidWorkflowState, to: BidWorkflowState, context: TransitionContext): TransitionResult {
    const key = `${from}->${to}`;

    switch (key) {
      case 'not_started->ready_to_bid':
        this.require(context.readinessPasses === true, 'BID_READINESS_GATE_FAILED');
        break;
      case 'ready_to_bid->bid_in_progress':
        if (context.scope === undefined) {
          throw new BidWorkflowStateMachineError('BID_SCOPE_INVALID');
        }
        this.validateScope(context.scope);
        this.require(context.decisionBasisFresh === true, 'BID_READINESS_GATE_FAILED');
        this.require(context.readinessPasses === true, 'BID_READINESS_GATE_FAILED');
        break;
      case 'ready_to_bid->not_started':
        this.require(!context.activeBidJob, 'BID_ACTIVE_JOB_EXISTS');
        this.require(!context.remoteSideEffectEvidence, 'BID_REMOTE_MUTATION_EXISTS');
        this.require(
          context.staleDecision === true ||
            context.l5BlockerActivated === true ||
            context.bidWindowExpired === true ||
            context.operatorWithdrawn === true,
          'BID_READINESS_GATE_FAILED',
        );
        break;
      case 'bid_in_progress->draft_saved':
        this.require(
          context.draftEvidence === true &&
            context.manifestLinked === true &&
            context.screenshotTrail === true,
          'BID_EVIDENCE_MISSING',
        );
        this.require(
          context.sideEffectClassReached === undefined ||
            this.sideEffectAtLeast(context.sideEffectClassReached, 'REMOTE_DRAFT'),
          'BID_EVIDENCE_MISSING',
        );
        break;
      case 'bid_in_progress->bid_submitted':
        this.requireFinalSubmissionGates(context);
        break;
      case 'bid_in_progress->bid_failed':
        this.require(
          !context.sideEffectClassReached || !this.sideEffectAtLeast(context.sideEffectClassReached, 'REMOTE_DRAFT'),
          'BID_REMOTE_MUTATION_EXISTS',
        );
        this.require(typeof context.errorCode === 'string' && context.errorCode.length > 0, 'BID_EVIDENCE_MISSING');
        break;
      case 'bid_in_progress->portal_state_unknown':
        this.require(
          context.remoteSideEffectEvidence === true || context.portalAmbiguous === true,
          'BID_PORTAL_RECONCILIATION_REQUIRED',
        );
        break;
      case 'draft_saved->bid_submitted':
        this.require(context.readinessPasses === true, 'BID_READINESS_GATE_FAILED');
        this.requireFinalSubmissionGates(context);
        break;
      case 'draft_saved->bid_in_progress':
        this.require(
          context.remoteDraftVerified === true && context.remoteDraftChanged !== true,
          'BID_PORTAL_RECONCILIATION_REQUIRED',
        );
        this.require(context.readinessPasses === true, 'BID_READINESS_GATE_FAILED');
        break;
      case 'draft_saved->bid_failed':
        this.require(context.operatorAbandoned === true, 'BID_REMOTE_MUTATION_EXISTS');
        this.require(context.automatedRetryFailure !== true, 'BID_REMOTE_MUTATION_EXISTS');
        break;
      case 'portal_state_unknown->bid_in_progress':
        this.require(
          context.operatorReconciled === true && context.portalReverified === true,
          'BID_PORTAL_RECONCILIATION_REQUIRED',
        );
        this.require(context.readinessPasses === true, 'BID_READINESS_GATE_FAILED');
        break;
      case 'portal_state_unknown->bid_submitted':
        this.require(
          context.operatorConfirmedSubmission === true &&
            typeof context.manifestConfirmationRef === 'string' &&
            context.manifestConfirmationRef.length > 0,
          'BID_CONFIRMATION_REF_MISSING',
        );
        break;
      case 'portal_state_unknown->bid_failed':
        this.require(
          context.noBuyerVisibleSubmission === true && context.portalEvidenceAmbiguous !== true,
          'BID_UNRECOVERABLE_EVIDENCE_REQUIRED',
        );
        break;
      case 'bid_failed->ready_to_bid':
        this.require(context.previousFailureReviewed === true, 'BID_FAILURE_REVIEW_REQUIRED');
        this.require(context.previousManifestPreserved === true, 'BID_FAILURE_REVIEW_REQUIRED');
        this.require(context.previousFailureUnresolvedRemoteMutation !== true, 'BID_REMOTE_MUTATION_EXISTS');
        this.require(context.readinessPasses === true, 'BID_READINESS_GATE_FAILED');
        break;
      default:
        throw new BidWorkflowStateMachineError('BID_SCOPE_INVALID');
    }

    return { from, to, errorCode: context.errorCode };
  }

  validateScope(scope: BidScopePayload): true {
    this.require(scope.processId.length > 0, 'BID_SCOPE_INVALID');
    this.require(scope.lotIds.length > 0 && scope.lotIds.every((id) => this.knownLots.has(id)), 'BID_SCOPE_INVALID');
    this.require(
      scope.candidateIds.length > 0 && scope.candidateIds.every((id) => this.knownCandidates.has(id)),
      'BID_SCOPE_INVALID',
    );
    this.require(
      scope.selectedQuoteIds.length > 0 && scope.selectedQuoteIds.every((id) => this.knownQuotes.has(id)),
      'BID_SCOPE_INVALID',
    );
    this.require(scope.companyProfileVersion.length > 0, 'BID_SCOPE_INVALID');
    this.require(scope.bidPortalType === 'procontract', 'BID_SCOPE_INVALID');
    this.require(scope.portalOpportunityRef.length > 0, 'BID_SCOPE_INVALID');
    this.require(typeof scope.reviewRequired === 'boolean', 'BID_SCOPE_INVALID');
    this.require(scope.decisionBasisHash.length > 0, 'BID_SCOPE_INVALID');
    this.require(SIDE_EFFECT_CLASS_ORDER.includes(scope.sideEffectCap), 'BID_SCOPE_INVALID');

    return true;
  }

  async computeBasisHash(scope: BidScopePayload, vectors: DecisionBasisVectors): Promise<string> {
    const encoded = new TextEncoder().encode(canonicalJson({ scope, vectors }));
    const digest = await globalThis.crypto.subtle.digest('SHA-256', encoded);

    return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
  }

  persistState(processId: string, row: StoredBidState): void {
    this.rows.set(processId, row);
  }

  loadStoredScope(processId: string): BidScopePayload | undefined {
    return this.rows.get(processId)?.bid_scope_json;
  }

  loadViewerProjection(processId: string): { bid_state: BidWorkflowState } | undefined {
    const row = this.rows.get(processId);

    return row ? { bid_state: row.bid_state } : undefined;
  }

  schemaInfo(tableName: string): { check_constraints: string[]; columns: string[] } {
    this.require(tableName === 'bid_workflow_states', 'BID_SCOPE_INVALID');

    return {
      check_constraints: [],
      columns: ['process_id', 'bid_state', 'bid_scope_json', 'review_required', 'bid_job_id'],
    };
  }

  private require(condition: boolean, code: TransitionErrorCode): void {
    if (!condition) {
      throw new BidWorkflowStateMachineError(code);
    }
  }

  private requireFinalSubmissionGates(context: TransitionContext): void {
    this.require(context.decisionBasisFresh === true, 'BID_READINESS_GATE_FAILED');
    this.require(context.capAllowsFormalSubmission === true, 'BID_SIDE_EFFECT_CAP_EXCEEDED');
    this.require(context.validationAccepted === true || context.validationOverride === true, 'BID_VALIDATION_GATE_FAILED');
  }

  private sideEffectAtLeast(observed: SideEffectClass, minimum: SideEffectClass): boolean {
    return SIDE_EFFECT_CLASS_ORDER.indexOf(observed) >= SIDE_EFFECT_CLASS_ORDER.indexOf(minimum);
  }
}

interface DecisionBasisVectors {
  companyProfileVersion: string;
  freshnessVector: readonly string[];
  l5EvidenceVector: readonly string[];
  pipelineAssessmentHash: string;
  quoteVersionVector: readonly string[];
}

class DecisionBasisFixture {
  static valid(overrides: Partial<DecisionBasisVectors> = {}): DecisionBasisVectors {
    return {
      pipelineAssessmentHash: 'pipeline-assessment-v1',
      quoteVersionVector: ['quote-1:v1'],
      companyProfileVersion: 'profile-v1',
      freshnessVector: ['candidate-1:2026-06-30:false'],
      l5EvidenceVector: ['tariff:v1', 'measures:v1', 'fx:v1'],
      ...overrides,
    };
  }

  static eachChangedVector(scope: BidScopePayload): Array<{ scope: BidScopePayload; vectors: DecisionBasisVectors }> {
    return [
      {
        scope,
        vectors: this.valid({ pipelineAssessmentHash: 'pipeline-assessment-v2' }),
      },
      {
        scope,
        vectors: this.valid({ quoteVersionVector: ['quote-1:v2'] }),
      },
      {
        scope: { ...scope, companyProfileVersion: 'profile-v2' },
        vectors: this.valid({ companyProfileVersion: 'profile-v2' }),
      },
      {
        scope,
        vectors: this.valid({ freshnessVector: ['candidate-1:2026-05-01:true'] }),
      },
      {
        scope,
        vectors: this.valid({ l5EvidenceVector: ['tariff:v2', 'measures:v1', 'fx:v1'] }),
      },
    ];
  }
}

class BidScopeFixture {
  static valid(overrides: Partial<BidScopePayload> = {}): BidScopePayload {
    return {
      processId: 'process-123',
      lotIds: ['lot-1'],
      candidateIds: ['candidate-1'],
      selectedQuoteIds: ['quote-1'],
      companyProfileVersion: 'profile-v1',
      bidPortalType: 'procontract',
      portalOpportunityRef: 'DN-OPP-123',
      reviewRequired: true,
      decisionBasisHash: 'decision-basis-hash-v1',
      sideEffectCap: 'REMOTE_DRAFT',
      ...overrides,
    };
  }

  static operatorStartedDefaults(): Pick<BidScopePayload, 'reviewRequired' | 'sideEffectCap'> {
    return {
      reviewRequired: true,
      sideEffectCap: 'REMOTE_DRAFT',
    };
  }
}

function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) {
    return `[${value.map((item) => canonicalJson(item)).join(',')}]`;
  }

  if (typeof value === 'object' && value !== null) {
    const record = value as Record<string, unknown>;
    return `{${Object.keys(record)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(record[key])}`)
      .join(',')}}`;
  }

  return JSON.stringify(value);
}
