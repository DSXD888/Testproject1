import { beforeEach, describe, expect, it } from 'vitest';
import type { BidScopePayload, CompanyProfile } from '../bid-scope-types';
import { validateCompanyProfile } from '../company-profile-schema';

type NonOverrideableBlockerCode =
  | 'OPERATOR_NOT_APPROVED'
  | 'DECISION_BASIS_STALE'
  | 'CLASSIFICATION_AMBIGUOUS'
  | 'TARIFF_BRANCH_DIVERGENCE'
  | 'SUPPLEMENTARY_UNIT_MISSING'
  | 'BROKER_REQUIRED'
  | 'CLASSIFICATION_FRESH_UNTIL_EXPIRED'
  | 'COMPANY_PROFILE_OUT_OF_DATE'
  | 'UPSTREAM_PIPELINE_NOT_VALIDATED'
  | 'PORTAL_OPPORTUNITY_REF_MISSING';

type OverrideableBlockerCode = 'QUOTE_ROLE_FLOOR_ONLY';

type BlockerCode = NonOverrideableBlockerCode | OverrideableBlockerCode;

interface ReadinessBlocker {
  code: BlockerCode;
  overrideable: boolean;
}

interface ReadinessOutcome {
  eligible: boolean;
  blockers: ReadinessBlocker[];
}

interface OperatorDecisionHead {
  disposition: 'approved' | 'pending' | 'rejected';
  reviewedBasisHash: string;
}

interface QuoteRoleOverride {
  reason: string;
  recordedBy: string;
  recordedAt: string;
}

interface TrustDebtAcceptance {
  processId: string;
  lotIds: string[];
  candidateIds: string[];
  reason: string;
}

const FLOOR_ONLY_ROLES = new Set(['domestic_wholesale_floor', 'optimistic_floor']);
const NOW = new Date('2026-05-22T12:00:00Z');

describe('L6-T-bid_readiness_gate', () => {
  let fixture: BidReadinessGateFixture;
  let scope: BidScopePayload;

  beforeEach(() => {
    fixture = new BidReadinessGateFixture();
    scope = BidScopeFixture.valid();
  });

  it('L6-T-bid_readiness_gate-001: pending_operator_intents ignored; only approved heads eligible', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedOperatorDecisionHead({ disposition: 'approved', reviewedBasisHash: scope.decisionBasisHash });
    fixture.seedPendingOperatorIntents({ approval: 'pending' });

    await expect(fixture.evaluate(scope)).resolves.toEqual({ eligible: true, blockers: [] });

    fixture.seedOperatorDecisionHead({ disposition: 'pending', reviewedBasisHash: scope.decisionBasisHash });
    const blocked = await fixture.evaluate(scope);

    expect(blocked.eligible).toBe(false);
    expect(blocked.blockers).toContainEqual({
      code: 'OPERATOR_NOT_APPROVED',
      overrideable: false,
    });
  });

  it('L6-T-bid_readiness_gate-002: stale reviewedBasisHash blocks with actionable diagnostic', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedOperatorDecisionHead({
      disposition: 'approved',
      reviewedBasisHash: 'stale-hash-aaa',
    });
    fixture.seedCurrentBasisHash('current-hash-bbb');

    const outcome = await fixture.evaluate(scope);

    expect(outcome.eligible).toBe(false);
    expect(outcome.blockers).toContainEqual({
      code: 'DECISION_BASIS_STALE',
      overrideable: false,
    });
  });

  it('L6-T-bid_readiness_gate-003: CLASSIFICATION_AMBIGUOUS blocks as non-overrideable', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedL5Blocker('CLASSIFICATION_AMBIGUOUS');

    const outcome = await fixture.evaluate(scope);

    expect(outcome.blockers).toContainEqual({
      code: 'CLASSIFICATION_AMBIGUOUS',
      overrideable: false,
    });
  });

  it('L6-T-bid_readiness_gate-004: TARIFF_BRANCH_DIVERGENCE blocks as non-overrideable', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedL5Blocker('TARIFF_BRANCH_DIVERGENCE');

    const outcome = await fixture.evaluate(scope);

    expect(outcome.blockers).toContainEqual({
      code: 'TARIFF_BRANCH_DIVERGENCE',
      overrideable: false,
    });
  });

  it('L6-T-bid_readiness_gate-005: SUPPLEMENTARY_UNIT_MISSING blocks as non-overrideable', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedL5Blocker('SUPPLEMENTARY_UNIT_MISSING');

    const outcome = await fixture.evaluate(scope);

    expect(outcome.blockers).toContainEqual({
      code: 'SUPPLEMENTARY_UNIT_MISSING',
      overrideable: false,
    });
  });

  it('L6-T-bid_readiness_gate-006: decision_grade=broker_required blocks as non-overrideable', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedDecisionGrade('broker_required');

    const outcome = await fixture.evaluate(scope);

    expect(outcome.blockers).toContainEqual({
      code: 'BROKER_REQUIRED',
      overrideable: false,
    });
  });

  it('L6-T-bid_readiness_gate-007: expired classification freshness blocks as non-overrideable', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedClassificationFresh({ candidateId: scope.candidateIds[0], expired: true });

    const outcome = await fixture.evaluate(scope);

    expect(outcome.blockers).toContainEqual({
      code: 'CLASSIFICATION_FRESH_UNTIL_EXPIRED',
      overrideable: false,
    });
  });

  it('L6-T-bid_readiness_gate-008: missing or expired company profile blocks as non-overrideable', async () => {
    fixture.seedAllGatesPass(scope);
    const expiredProfile = CompanyProfileFixture.valid({
      certifications: [{ type: 'ISO9001', number: 'ISO-1', expiryDate: '2026-01-01', issuingBody: 'BSI' }],
    });

    expect(validateCompanyProfile(expiredProfile, NOW)).toContainEqual({
      kind: 'cert_expired',
      certType: 'ISO9001',
      expiryDate: '2026-01-01',
    });

    fixture.seedCompanyProfile(expiredProfile);
    const outcome = await fixture.evaluate(scope);

    expect(outcome.blockers).toContainEqual({
      code: 'COMPANY_PROFILE_OUT_OF_DATE',
      overrideable: false,
    });
  });

  it('L6-T-bid_readiness_gate-009: floor-only quote roles block without override', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedQuoteRoles(['domestic_wholesale_floor', 'optimistic_floor']);

    const outcome = await fixture.evaluate(scope);

    expect(outcome.eligible).toBe(false);
    expect(outcome.blockers).toContainEqual({
      code: 'QUOTE_ROLE_FLOOR_ONLY',
      overrideable: true,
    });
  });

  it('L6-T-bid_readiness_gate-010: quote-role override allows progress with structured acknowledgment', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedQuoteRoles(['domestic_wholesale_floor']);
    fixture.seedQuoteRoleOverride({
      reason: 'Sole-source pricing reviewed; bid intent acknowledged for this scope.',
      recordedBy: 'operator',
      recordedAt: '2026-05-22T12:00:00Z',
    });

    const outcome = await fixture.evaluate(scope);

    expect(outcome).toEqual({ eligible: true, blockers: [] });
    expect(fixture.quoteRoleOverride?.reason).toContain('acknowledged');
  });

  it('L6-T-bid_readiness_gate-011: upstream trust state blocks unless acceptance is scope-matched', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedPipelineAssessmentValidationState('suspect');

    await expect(fixture.evaluate(scope)).resolves.toMatchObject({
      eligible: false,
      blockers: [{ code: 'UPSTREAM_PIPELINE_NOT_VALIDATED', overrideable: false }],
    });

    fixture.seedTrustDebtAcceptance({
      processId: scope.processId,
      lotIds: scope.lotIds,
      candidateIds: scope.candidateIds,
      reason: 'Reviewed and accepting trust debt for this exact scope.',
    });

    await expect(fixture.evaluate(scope)).resolves.toEqual({ eligible: true, blockers: [] });

    fixture.seedTrustDebtAcceptance({
      processId: scope.processId,
      lotIds: scope.lotIds,
      candidateIds: ['candidate-other'],
      reason: 'Different candidate scope.',
    });

    await expect(fixture.evaluate(scope)).resolves.toMatchObject({
      eligible: false,
      blockers: [{ code: 'UPSTREAM_PIPELINE_NOT_VALIDATED', overrideable: false }],
    });
  });

  it('L6-T-bid_readiness_gate-012: fully green candidate becomes eligible', async () => {
    fixture.seedAllGatesPass(scope);

    await expect(fixture.evaluate(scope)).resolves.toEqual({ eligible: true, blockers: [] });
  });

  it('L6-T-bid_readiness_gate-013: readiness is recomputed and stale cached eligibility rejected', async () => {
    fixture.seedAllGatesPass(scope);

    await expect(fixture.evaluate(scope)).resolves.toEqual({ eligible: true, blockers: [] });

    fixture.seedL5Blocker('CLASSIFICATION_AMBIGUOUS');
    const recomputed = await fixture.evaluate(scope);

    expect(recomputed.eligible).toBe(false);
    expect(fixture.usedCachedEvaluation).toBe(false);
    expect(recomputed.blockers).toContainEqual({
      code: 'CLASSIFICATION_AMBIGUOUS',
      overrideable: false,
    });
  });

  it('L6-T-bid_readiness_gate-014: missing portalOpportunityRef blocks before side effects', async () => {
    fixture.seedAllGatesPass(scope);
    const noRef = { ...scope, portalOpportunityRef: '' };

    const outcome = await fixture.evaluate(noRef);

    expect(outcome.blockers).toContainEqual({
      code: 'PORTAL_OPPORTUNITY_REF_MISSING',
      overrideable: false,
    });
  });

  it('L6-T-bid_readiness_gate-015: multiple blockers are reported together without short-circuiting', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedL5Blocker('CLASSIFICATION_AMBIGUOUS');
    fixture.seedClassificationFresh({ candidateId: scope.candidateIds[0], expired: true });
    fixture.seedQuoteRoles(['domestic_wholesale_floor']);

    const outcome = await fixture.evaluate(scope);

    expect(outcome.blockers).toEqual(
      expect.arrayContaining([
        { code: 'CLASSIFICATION_AMBIGUOUS', overrideable: false },
        { code: 'CLASSIFICATION_FRESH_UNTIL_EXPIRED', overrideable: false },
        { code: 'QUOTE_ROLE_FLOOR_ONLY', overrideable: true },
      ]),
    );
    expect(outcome.blockers).toHaveLength(3);
  });

  it('L6-T-bid_readiness_gate-016: overrides cannot clear non-overrideable gates and are logged as rejected', async () => {
    fixture.seedAllGatesPass(scope);
    fixture.seedL5Blocker('CLASSIFICATION_AMBIGUOUS');
    fixture.attemptOverride({
      kind: 'classification_ambiguous_override',
      reason: 'I want to bid anyway.',
    });

    const outcome = await fixture.evaluate(scope);

    expect(outcome.eligible).toBe(false);
    expect(outcome.blockers).toContainEqual({
      code: 'CLASSIFICATION_AMBIGUOUS',
      overrideable: false,
    });
    expect(fixture.rejectedOverrideAcknowledgments).toContainEqual({
      kind: 'classification_ambiguous_override',
      reason: 'I want to bid anyway.',
    });
  });
});

class BidReadinessGateFixture {
  private operatorDecisionHead: OperatorDecisionHead = {
    disposition: 'approved',
    reviewedBasisHash: BidScopeFixture.valid().decisionBasisHash,
  };

  private currentBasisHash = BidScopeFixture.valid().decisionBasisHash;
  private readonly l5Blockers = new Set<NonOverrideableBlockerCode>();
  private decisionGrade: 'decision_ready' | 'broker_required' = 'decision_ready';
  private readonly expiredClassificationCandidates = new Set<string>();
  private companyProfile: CompanyProfile = CompanyProfileFixture.valid();
  private quoteRoles: string[] = ['market_benchmark'];
  private pipelineAssessmentValidationState: 'validated' | 'suspect' | 'rejected' = 'validated';
  private trustDebtAcceptance: TrustDebtAcceptance | null = null;
  private pendingOperatorIntents: Readonly<Record<string, unknown>> | null = null;

  quoteRoleOverride: QuoteRoleOverride | null = null;
  rejectedOverrideAcknowledgments: Array<{ kind: string; reason: string }> = [];
  usedCachedEvaluation = false;

  seedAllGatesPass(scope: BidScopePayload = BidScopeFixture.valid()): void {
    this.operatorDecisionHead = {
      disposition: 'approved',
      reviewedBasisHash: scope.decisionBasisHash,
    };
    this.currentBasisHash = scope.decisionBasisHash;
    this.l5Blockers.clear();
    this.decisionGrade = 'decision_ready';
    this.expiredClassificationCandidates.clear();
    this.companyProfile = CompanyProfileFixture.valid();
    this.quoteRoles = ['market_benchmark'];
    this.quoteRoleOverride = null;
    this.pipelineAssessmentValidationState = 'validated';
    this.trustDebtAcceptance = null;
    this.pendingOperatorIntents = null;
    this.rejectedOverrideAcknowledgments = [];
    this.usedCachedEvaluation = false;
  }

  seedOperatorDecisionHead(head: OperatorDecisionHead): void {
    this.operatorDecisionHead = head;
  }

  seedPendingOperatorIntents(intent: Readonly<Record<string, unknown>>): void {
    this.pendingOperatorIntents = intent;
  }

  seedCurrentBasisHash(hash: string): void {
    this.currentBasisHash = hash;
  }

  seedL5Blocker(code: NonOverrideableBlockerCode): void {
    this.l5Blockers.add(code);
  }

  seedDecisionGrade(grade: 'decision_ready' | 'broker_required'): void {
    this.decisionGrade = grade;
  }

  seedClassificationFresh(input: { candidateId: string | undefined; expired: boolean }): void {
    if (input.candidateId && input.expired) {
      this.expiredClassificationCandidates.add(input.candidateId);
    }
  }

  seedCompanyProfile(profile: CompanyProfile): void {
    this.companyProfile = profile;
  }

  seedQuoteRoles(roles: string[]): void {
    this.quoteRoles = roles;
  }

  seedQuoteRoleOverride(override: QuoteRoleOverride): void {
    this.quoteRoleOverride = override;
  }

  seedPipelineAssessmentValidationState(state: 'validated' | 'suspect' | 'rejected'): void {
    this.pipelineAssessmentValidationState = state;
  }

  seedTrustDebtAcceptance(acceptance: TrustDebtAcceptance): void {
    this.trustDebtAcceptance = acceptance;
  }

  attemptOverride(override: { kind: string; reason: string }): void {
    this.rejectedOverrideAcknowledgments.push(override);
  }

  async evaluate(scope: BidScopePayload): Promise<ReadinessOutcome> {
    this.usedCachedEvaluation = false;
    void this.pendingOperatorIntents;

    const blockers: ReadinessBlocker[] = [];

    if (scope.portalOpportunityRef.trim().length === 0) {
      blockers.push({ code: 'PORTAL_OPPORTUNITY_REF_MISSING', overrideable: false });
    }

    if (this.operatorDecisionHead.disposition !== 'approved') {
      blockers.push({ code: 'OPERATOR_NOT_APPROVED', overrideable: false });
    }

    if (this.operatorDecisionHead.reviewedBasisHash !== this.currentBasisHash) {
      blockers.push({ code: 'DECISION_BASIS_STALE', overrideable: false });
    }

    for (const code of this.l5Blockers) {
      blockers.push({ code, overrideable: false });
    }

    if (this.decisionGrade === 'broker_required') {
      blockers.push({ code: 'BROKER_REQUIRED', overrideable: false });
    }

    if (scope.candidateIds.some((candidateId) => this.expiredClassificationCandidates.has(candidateId))) {
      blockers.push({ code: 'CLASSIFICATION_FRESH_UNTIL_EXPIRED', overrideable: false });
    }

    if (validateCompanyProfile(this.companyProfile, NOW).length > 0) {
      blockers.push({ code: 'COMPANY_PROFILE_OUT_OF_DATE', overrideable: false });
    }

    if (this.pipelineAssessmentValidationState !== 'validated' && !this.hasScopeMatchedTrustDebt(scope)) {
      blockers.push({ code: 'UPSTREAM_PIPELINE_NOT_VALIDATED', overrideable: false });
    }

    if (this.isFloorOnlyQuoteEvidence() && this.quoteRoleOverride === null) {
      blockers.push({ code: 'QUOTE_ROLE_FLOOR_ONLY', overrideable: true });
    }

    return { eligible: blockers.length === 0, blockers };
  }

  private isFloorOnlyQuoteEvidence(): boolean {
    return this.quoteRoles.length > 0 && this.quoteRoles.every((role) => FLOOR_ONLY_ROLES.has(role));
  }

  private hasScopeMatchedTrustDebt(scope: BidScopePayload): boolean {
    if (this.trustDebtAcceptance === null) {
      return false;
    }

    return (
      this.trustDebtAcceptance.processId === scope.processId &&
      sameMembers(this.trustDebtAcceptance.lotIds, scope.lotIds) &&
      sameMembers(this.trustDebtAcceptance.candidateIds, scope.candidateIds) &&
      this.trustDebtAcceptance.reason.trim().length > 0
    );
  }
}

class BidScopeFixture {
  static valid(): BidScopePayload {
    return {
      processId: 'process-l6-001',
      lotIds: ['lot-1'],
      candidateIds: ['candidate-1'],
      selectedQuoteIds: ['quote-1'],
      companyProfileVersion: 'company-profile-v1',
      bidPortalType: 'procontract',
      portalOpportunityRef: 'DN-OPP-123',
      reviewRequired: true,
      decisionBasisHash: '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
      sideEffectCap: 'REMOTE_DRAFT',
    };
  }
}

class CompanyProfileFixture {
  static valid(overrides: Partial<CompanyProfile> = {}): CompanyProfile {
    return {
      companyName: 'Sorelon Ltd',
      registrationNumber: '12345678',
      vatNumber: 'GB123456789',
      address: {
        line1: '1 Procurement Street',
        city: 'London',
        postcode: 'SW1A 1AA',
        country: 'United Kingdom',
      },
      primaryContact: {
        name: 'Operator Contact',
        email: 'operator@example.test',
        phone: '+442000000000',
        role: 'Bid Manager',
      },
      certifications: [
        {
          type: 'ISO9001',
          number: 'ISO-9001-2026',
          expiryDate: '2027-05-22',
          issuingBody: 'BSI',
        },
      ],
      insurancePolicies: [
        {
          type: 'public_liability',
          provider: 'Example Insurer',
          policyNumber: 'PL-2026',
          coverAmountPence: 5_000_000_00,
          expiryDate: '2027-05-22',
        },
      ],
      financialStatements: {
        lastFiledYear: 2025,
        companiesHouseRef: 'CH-2025',
      },
      smeStatus: 'small',
      ...overrides,
    };
  }
}

function sameMembers(left: string[], right: string[]): boolean {
  if (left.length !== right.length) {
    return false;
  }

  const rightValues = new Set(right);

  return left.every((value) => rightValues.has(value));
}
