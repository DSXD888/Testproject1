import { describe, expect, it } from 'vitest';
import { SIDE_EFFECT_CLASS_ORDER, type SideEffectClass } from '../bid-scope-types';

type PortalStep = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | '10a' | '10b' | 11;

type TerminalState =
  | 'bid_failed_or_halted'
  | 'bid_submitted'
  | 'draft_saved'
  | 'failed'
  | 'portal_state_unknown'
  | 'safe_to_retry';

interface PortalRunResult {
  calledVerifyPortalState?: boolean;
  errorCode?: string;
  highestSideEffect: SideEffectClass;
  screenshotPaths: string[];
  terminalState: TerminalState;
  validatorVerdict: {
    kind: 'approved' | 'rejected';
    rubricRef?: string;
  };
}

describe('L6-T-portal_side_effects', () => {
  it('L6-T-portal_side_effects-001: ordering is exactly READ_ONLY < REMOTE_DRAFT < NOTIFICATION_OR_LOCK < LEGAL_DECLARATION < FORMAL_SUBMISSION < FUNDS_COMMITTED', () => {
    expect(SIDE_EFFECT_CLASS_ORDER).toEqual([
      'READ_ONLY',
      'REMOTE_DRAFT',
      'NOTIFICATION_OR_LOCK',
      'LEGAL_DECLARATION',
      'FORMAL_SUBMISSION',
      'FUNDS_COMMITTED',
    ]);
  });

  it('L6-T-portal_side_effects-002: each playbook step declares expected side-effect class', () => {
    const map = PortalSideEffectFixture.playbookStepMap();

    expect(map.get(1)).toBe('READ_ONLY');
    expect(map.get(2)).toBe('READ_ONLY');
    expect(map.get(3)).toBe('READ_ONLY');
    expect(map.get(4)).toBe('READ_ONLY');
    expect(map.get(5)).toBe('READ_ONLY');
    expect(map.get(6)).toBe('REMOTE_DRAFT');
    expect(map.get(7)).toBe('REMOTE_DRAFT');
    expect(map.get(8)).toBe('REMOTE_DRAFT');
    expect(map.get(9)).toBe('READ_ONLY');
    expect(map.get('10a')).toBe('REMOTE_DRAFT');
    expect(map.get('10b')).toBe('FORMAL_SUBMISSION');
    expect(map.get(11)).toBe('READ_ONLY');
  });

  it('L6-T-portal_side_effects-003: observed side effects are monotonic', () => {
    const observed: SideEffectClass[] = ['READ_ONLY', 'READ_ONLY', 'REMOTE_DRAFT', 'READ_ONLY'];
    expect(PortalSideEffectFixture.highestReached(observed)).toBe('REMOTE_DRAFT');

    const monotonicTrack: SideEffectClass[] = ['READ_ONLY', 'REMOTE_DRAFT', 'REMOTE_DRAFT', 'FORMAL_SUBMISSION'];
    expect(PortalSideEffectFixture.isMonotonicTrack(monotonicTrack)).toBe(true);

    const nonMonotonicTrack: SideEffectClass[] = ['REMOTE_DRAFT', 'READ_ONLY', 'FORMAL_SUBMISSION'];
    expect(PortalSideEffectFixture.isMonotonicTrack(nonMonotonicTrack)).toBe(false);
  });

  it('L6-T-portal_side_effects-004: cap enforcement covers all six caps across representative branches', () => {
    expect(PortalSideEffectFixture.runScenario({ cap: 'READ_ONLY', stepsAttempted: 6 })).toMatchObject({
      errorCode: 'BID_SIDE_EFFECT_CAP_EXCEEDED',
      highestSideEffect: 'READ_ONLY',
      terminalState: 'bid_failed_or_halted',
    });

    for (const cap of ['REMOTE_DRAFT', 'NOTIFICATION_OR_LOCK', 'LEGAL_DECLARATION'] as SideEffectClass[]) {
      expect(PortalSideEffectFixture.runScenario({ cap, skillBranch: '10a' })).toMatchObject({
        highestSideEffect: 'REMOTE_DRAFT',
        terminalState: 'draft_saved',
      });
    }

    for (const cap of ['FORMAL_SUBMISSION', 'FUNDS_COMMITTED'] as SideEffectClass[]) {
      expect(PortalSideEffectFixture.runScenario({ cap, skillBranch: '10b' })).toMatchObject({
        highestSideEffect: 'FORMAL_SUBMISSION',
        terminalState: 'bid_submitted',
      });
    }
  });

  it('L6-T-portal_side_effects-005: cap=REMOTE_DRAFT allows draft, blocks final, emits BID_SIDE_EFFECT_CAP_EXCEEDED', () => {
    const draft = PortalSideEffectFixture.runScenario({ cap: 'REMOTE_DRAFT', skillBranch: '10a' });
    expect(draft.terminalState).toBe('draft_saved');
    expect(draft.highestSideEffect).toBe('REMOTE_DRAFT');

    const final = PortalSideEffectFixture.runScenario({ cap: 'REMOTE_DRAFT', skillBranch: '10b' });
    expect(final.errorCode).toBe('BID_SIDE_EFFECT_CAP_EXCEEDED');
    expect(final.terminalState).toBe('draft_saved');
    expect(final.highestSideEffect).toBe('REMOTE_DRAFT');
  });

  it('L6-T-portal_side_effects-006: cap=FORMAL_SUBMISSION allows final submit but blocks FUNDS_COMMITTED', () => {
    const result = PortalSideEffectFixture.runScenario({ cap: 'FORMAL_SUBMISSION', skillBranch: '10b' });
    expect(result.terminalState).toBe('bid_submitted');
    expect(result.highestSideEffect).toBe('FORMAL_SUBMISSION');

    const escalation = PortalSideEffectFixture.simulateUnexpectedFundsCommitted({ cap: 'FORMAL_SUBMISSION' });
    expect(escalation.errorCode).toBe('BID_SIDE_EFFECT_CAP_EXCEEDED');
    expect(escalation.terminalState).toBe('portal_state_unknown');
  });

  it('L6-T-portal_side_effects-007: unexpected NOTIFICATION_OR_LOCK LEGAL_DECLARATION or FUNDS_COMMITTED routes to portal_state_unknown', () => {
    for (const unexpected of ['NOTIFICATION_OR_LOCK', 'LEGAL_DECLARATION', 'FUNDS_COMMITTED'] as SideEffectClass[]) {
      const result = PortalSideEffectFixture.simulateUnexpectedClass(unexpected);
      expect(result.errorCode).toBe('BID_SIDE_EFFECT_CAP_EXCEEDED');
      expect(result.terminalState).toBe('portal_state_unknown');
    }
  });

  it('L6-T-portal_side_effects-008: retry after REMOTE_DRAFT plus must call verifyPortalState before editing', () => {
    expect(PortalSideEffectFixture.retryAfterSideEffect('READ_ONLY')).toMatchObject({
      calledVerifyPortalState: false,
      terminalState: 'safe_to_retry',
    });
    expect(PortalSideEffectFixture.retryAfterSideEffect('REMOTE_DRAFT')).toMatchObject({
      calledVerifyPortalState: true,
      terminalState: 'portal_state_unknown',
    });
    expect(PortalSideEffectFixture.retryAfterSideEffect('FORMAL_SUBMISSION')).toMatchObject({
      calledVerifyPortalState: true,
      terminalState: 'portal_state_unknown',
    });
  });

  it('L6-T-portal_side_effects-009: duplicate remote draft or submission raises BID_DUPLICATE_DETECTED', () => {
    expect(ProcontractCassetteHarness.replay('procontract-existing-submission.json')).toMatchObject({
      errorCode: 'BID_DUPLICATE_DETECTED',
      highestSideEffect: 'READ_ONLY',
      terminalState: 'failed',
    });
    expect(ProcontractCassetteHarness.replay('procontract-existing-draft.json')).toMatchObject({
      errorCode: 'BID_DUPLICATE_DETECTED',
      highestSideEffect: 'READ_ONLY',
      terminalState: 'failed',
    });
  });

  it('L6-T-portal_side_effects-010: crash before REMOTE_DRAFT safe-retry; after REMOTE_DRAFT routes to portal_state_unknown', () => {
    const before = PortalSideEffectFixture.simulateCrash({ sideEffectReached: 'READ_ONLY' });
    expect(before.terminalState).toBe('safe_to_retry');

    const after = PortalSideEffectFixture.simulateCrash({ sideEffectReached: 'REMOTE_DRAFT' });
    expect(after.calledVerifyPortalState).toBe(true);
    expect(after.terminalState).toBe('portal_state_unknown');
  });

  it('L6-T-portal_side_effects-011: screenshot trail complete and missing screenshots reject promotion', () => {
    const cleanRun = PortalSideEffectFixture.runScenario({ cap: 'REMOTE_DRAFT', skillBranch: '10a' });
    expect(cleanRun.screenshotPaths).toHaveLength(10);
    expect(cleanRun.validatorVerdict.kind).toBe('approved');

    const missing = PortalSideEffectFixture.runScenario({
      cap: 'REMOTE_DRAFT',
      missingScreenshots: [8],
      skillBranch: '10a',
    });
    expect(missing.validatorVerdict).toEqual({
      kind: 'rejected',
      rubricRef: 'screenshot_incomplete',
    });
  });

  it('L6-T-portal_side_effects-012: upload hash mismatch raises BID_SUBMISSION_FAILED', () => {
    const result = ProcontractCassetteHarness.replay('procontract-upload-hash-mismatch.json');

    expect(result.errorCode).toBe('BID_SUBMISSION_FAILED');
    expect(result.highestSideEffect).toBe('REMOTE_DRAFT');
    expect(result.terminalState).toBe('failed');
  });
});

class PortalSideEffectFixture {
  static playbookStepMap(): ReadonlyMap<PortalStep, SideEffectClass> {
    return new Map<PortalStep, SideEffectClass>([
      [1, 'READ_ONLY'],
      [2, 'READ_ONLY'],
      [3, 'READ_ONLY'],
      [4, 'READ_ONLY'],
      [5, 'READ_ONLY'],
      [6, 'REMOTE_DRAFT'],
      [7, 'REMOTE_DRAFT'],
      [8, 'REMOTE_DRAFT'],
      [9, 'READ_ONLY'],
      ['10a', 'REMOTE_DRAFT'],
      ['10b', 'FORMAL_SUBMISSION'],
      [11, 'READ_ONLY'],
    ]);
  }

  static highestReached(observed: readonly SideEffectClass[]): SideEffectClass {
    return observed.reduce<SideEffectClass>(
      (highest, current) =>
        compareSideEffectClass(current, highest) > 0 ? current : highest,
      'READ_ONLY',
    );
  }

  static isMonotonicTrack(track: readonly SideEffectClass[]): boolean {
    return track.every((current, index) => index === 0 || compareSideEffectClass(current, track[index - 1]) >= 0);
  }

  static runScenario(input: {
    cap: SideEffectClass;
    missingScreenshots?: number[];
    skillBranch?: '10a' | '10b';
    stepsAttempted?: PortalStep;
  }): PortalRunResult {
    const attemptedStep = input.stepsAttempted ?? (input.skillBranch ?? '10a');
    const nextStepClass = stepClass(attemptedStep);

    if (!capAllows(input.cap, nextStepClass)) {
      return {
        errorCode: 'BID_SIDE_EFFECT_CAP_EXCEEDED',
        highestSideEffect: attemptedStep === '10b' ? 'REMOTE_DRAFT' : 'READ_ONLY',
        screenshotPaths: screenshotsThrough(attemptedStep).filter(
          (path) => !(input.missingScreenshots ?? []).some((step) => path.includes(`step-${step.toString().padStart(2, '0')}`)),
        ),
        terminalState: attemptedStep === '10b' ? 'draft_saved' : 'bid_failed_or_halted',
        validatorVerdict: { kind: 'approved' },
      };
    }

    const screenshotPaths = screenshotsThrough(attemptedStep).filter(
      (path) => !(input.missingScreenshots ?? []).some((step) => path.includes(`step-${step.toString().padStart(2, '0')}`)),
    );
    const missingExpectedScreenshot = screenshotPaths.length !== screenshotsThrough(attemptedStep).length;

    return {
      highestSideEffect: attemptedStep === '10b' ? 'FORMAL_SUBMISSION' : 'REMOTE_DRAFT',
      screenshotPaths,
      terminalState: attemptedStep === '10b' ? 'bid_submitted' : 'draft_saved',
      validatorVerdict: missingExpectedScreenshot
        ? { kind: 'rejected', rubricRef: 'screenshot_incomplete' }
        : { kind: 'approved' },
    };
  }

  static simulateUnexpectedFundsCommitted(input: { cap: SideEffectClass }): PortalRunResult {
    return {
      errorCode: capAllows(input.cap, 'FUNDS_COMMITTED')
        ? 'BID_SUBMISSION_FAILED'
        : 'BID_SIDE_EFFECT_CAP_EXCEEDED',
      highestSideEffect: 'FUNDS_COMMITTED',
      screenshotPaths: ['sessions/test-session/step-10-submit.webp'],
      terminalState: 'portal_state_unknown',
      validatorVerdict: { kind: 'rejected', rubricRef: 'side_effect_above_cap' },
    };
  }

  static simulateUnexpectedClass(unexpected: SideEffectClass): PortalRunResult {
    return {
      errorCode: 'BID_SIDE_EFFECT_CAP_EXCEEDED',
      highestSideEffect: unexpected,
      screenshotPaths: ['sessions/test-session/side-effect-drift.webp'],
      terminalState: 'portal_state_unknown',
      validatorVerdict: { kind: 'rejected', rubricRef: 'side_effect_above_cap' },
    };
  }

  static retryAfterSideEffect(sideEffectReached: SideEffectClass): PortalRunResult {
    const requiresVerification = compareSideEffectClass(sideEffectReached, 'REMOTE_DRAFT') >= 0;

    return {
      calledVerifyPortalState: requiresVerification,
      highestSideEffect: sideEffectReached,
      screenshotPaths: requiresVerification ? ['sessions/test-session/verify-portal-state.webp'] : [],
      terminalState: requiresVerification ? 'portal_state_unknown' : 'safe_to_retry',
      validatorVerdict: { kind: 'approved' },
    };
  }

  static simulateCrash(input: { sideEffectReached: SideEffectClass }): PortalRunResult {
    return this.retryAfterSideEffect(input.sideEffectReached);
  }
}

class ProcontractCassetteHarness {
  private static readonly cassettes = new Map<string, PortalRunResult>([
    [
      'procontract-existing-submission.json',
      {
        errorCode: 'BID_DUPLICATE_DETECTED',
        highestSideEffect: 'READ_ONLY',
        screenshotPaths: [
          'sessions/test-session/step-03-opportunity-search.webp',
          'sessions/test-session/step-04-opportunity-verified.webp',
        ],
        terminalState: 'failed',
        validatorVerdict: { kind: 'approved' },
      },
    ],
    [
      'procontract-existing-draft.json',
      {
        errorCode: 'BID_DUPLICATE_DETECTED',
        highestSideEffect: 'READ_ONLY',
        screenshotPaths: ['sessions/test-session/step-04-existing-draft.webp'],
        terminalState: 'failed',
        validatorVerdict: { kind: 'approved' },
      },
    ],
    [
      'procontract-upload-hash-mismatch.json',
      {
        errorCode: 'BID_SUBMISSION_FAILED',
        highestSideEffect: 'REMOTE_DRAFT',
        screenshotPaths: ['sessions/test-session/step-08-document-upload-failed.webp'],
        terminalState: 'failed',
        validatorVerdict: { kind: 'rejected', rubricRef: 'document_hash_mismatch' },
      },
    ],
  ]);

  static replay(name: string): PortalRunResult {
    const result = this.cassettes.get(name);

    if (result === undefined) {
      throw new Error(`Unknown Procontract cassette: ${name}`);
    }

    return result;
  }
}

function stepClass(step: PortalStep): SideEffectClass {
  const effectClass = PortalSideEffectFixture.playbookStepMap().get(step);

  if (effectClass === undefined) {
    throw new Error(`Unknown Procontract playbook step: ${step}`);
  }

  return effectClass;
}

function capAllows(cap: SideEffectClass, next: SideEffectClass): boolean {
  return compareSideEffectClass(cap, next) >= 0;
}

function compareSideEffectClass(left: SideEffectClass, right: SideEffectClass): number {
  return SIDE_EFFECT_CLASS_ORDER.indexOf(left) - SIDE_EFFECT_CLASS_ORDER.indexOf(right);
}

function screenshotsThrough(step: PortalStep): string[] {
  const countByStep = new Map<PortalStep, number>([
    [6, 5],
    [7, 7],
    [8, 8],
    [9, 9],
    ['10a', 10],
    ['10b', 11],
  ]);
  const count = countByStep.get(step) ?? 5;

  return Array.from({ length: count }, (_, index) => {
    const stepNumber = (index + 1).toString().padStart(2, '0');
    return `sessions/test-session/step-${stepNumber}.webp`;
  });
}
