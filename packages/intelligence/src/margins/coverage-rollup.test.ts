import { describe, expect, it } from "bun:test";

import {
  AUTO_QUALIFICATION_VALUE_FLOOR_BPS,
  computeCoverageRollup,
  describeCoverageThreshold,
  isAutoQualificationReady,
  type CoverageInput,
} from "./coverage-rollup";

describe("computeCoverageRollup", () => {
  it("TS-COV-001: follows qualifying-lot denominators for lot-count metrics", () => {
    const output = computeCoverageRollup({
      lots: [
        {
          lotId: "lot-a",
          qualificationResult: "pass",
          isMandatory: true,
          lotAddressableValuePence: 60_000,
          candidates: [
            {
              candidateId: "cand-a1",
              hasPricing: true,
              hasLandedCost: true,
              passesNetGate: true,
              candidateValueShareBps: 10_000,
            },
          ],
        },
        {
          lotId: "lot-b",
          qualificationResult: "review",
          isMandatory: true,
          lotAddressableValuePence: 30_000,
          candidates: [
            {
              candidateId: "cand-b1",
              hasPricing: true,
              hasLandedCost: false,
              passesNetGate: false,
              candidateValueShareBps: 10_000,
            },
          ],
        },
        {
          lotId: "lot-c",
          qualificationResult: "pass",
          isMandatory: false,
          lotAddressableValuePence: 10_000,
          candidates: [],
        },
        {
          lotId: "lot-d",
          qualificationResult: "reject",
          isMandatory: false,
          lotAddressableValuePence: 25_000,
          candidates: [
            {
              candidateId: "cand-d1",
              hasPricing: true,
              hasLandedCost: true,
              passesNetGate: true,
              candidateValueShareBps: 10_000,
            },
          ],
        },
      ],
    });

    expect(output.pricedCoverageBps).toBe(6667);
    expect(output.landedCostCoverageBps).toBe(3333);
    expect(output.netQualifiedCoverageBps).toBe(3333);
  });

  it("TS-COV-002: uses qualifying lot value rather than lot count for value-weighted metrics", () => {
    const output = computeCoverageRollup({
      lots: [
        {
          lotId: "lot-a",
          qualificationResult: "pass",
          isMandatory: true,
          lotAddressableValuePence: 80_000,
          candidates: [
            {
              candidateId: "cand-a1",
              hasPricing: true,
              hasLandedCost: true,
              passesNetGate: false,
              candidateValueShareBps: 5_000,
            },
          ],
        },
        {
          lotId: "lot-b",
          qualificationResult: "review",
          isMandatory: true,
          lotAddressableValuePence: 20_000,
          candidates: [
            {
              candidateId: "cand-b1",
              hasPricing: true,
              hasLandedCost: true,
              passesNetGate: true,
              candidateValueShareBps: 10_000,
            },
          ],
        },
      ],
    });

    expect(output.pricedValueCoverageBps).toBe(6000);
    expect(output.landedCostValueCoverageBps).toBe(6000);
    expect(output.netQualifiedValueCoverageBps).toBe(2000);
    expect(isAutoQualificationReady(output)).toBe(false);
  });

  it("TS-COV-003: maps threshold bands to stable tone and label pairs", () => {
    expect(describeCoverageThreshold(7000)).toEqual({
      tone: "green",
      label: "Strong coverage",
    });
    expect(describeCoverageThreshold(6999)).toEqual({
      tone: "amber",
      label: "Partial coverage -- review recommended",
    });
    expect(describeCoverageThreshold(3000)).toEqual({
      tone: "amber",
      label: "Partial coverage -- review recommended",
    });
    expect(describeCoverageThreshold(2999)).toEqual({
      tone: "red",
      label: "Low coverage -- manual review required",
    });
  });

  it("TS-COV-004: returns unresolved mandatory lots as an absolute blocker count", () => {
    const output = computeCoverageRollup({
      lots: [
        {
          lotId: "lot-a",
          qualificationResult: "pass",
          isMandatory: true,
          lotAddressableValuePence: 50_000,
          candidates: [],
        },
        {
          lotId: "lot-b",
          qualificationResult: "review",
          isMandatory: true,
          lotAddressableValuePence: 50_000,
          candidates: [
            {
              candidateId: "cand-b1",
              hasPricing: true,
              hasLandedCost: true,
              passesNetGate: true,
              candidateValueShareBps: 10_000,
            },
          ],
        },
      ],
    });

    expect(output.unresolvedMandatoryLots).toBe(1);
    expect(output.unresolvedMandatoryLots).not.toBeGreaterThanOrEqual(10_000);
  });

  it("TS-COV-005: keeps value coverage conservative when lot values or candidate shares are missing", () => {
    const knownValueOutput = computeCoverageRollup({
      lots: [
        {
          lotId: "lot-a",
          qualificationResult: "pass",
          isMandatory: true,
          lotAddressableValuePence: 50_000,
          candidates: [
            {
              candidateId: "cand-a1",
              hasPricing: true,
              hasLandedCost: true,
              passesNetGate: true,
              candidateValueShareBps: 5_000,
            },
          ],
        },
        {
          lotId: "lot-b",
          qualificationResult: "review",
          isMandatory: true,
          lotAddressableValuePence: 50_000,
          candidates: [
            {
              candidateId: "cand-b1",
              hasPricing: true,
              hasLandedCost: true,
              passesNetGate: true,
              candidateValueShareBps: null,
            },
          ],
        },
      ],
    });

    expect(knownValueOutput.netQualifiedValueCoverageBps).toBe(2500);

    const missingValueOutput = computeCoverageRollup({
      lots: [
        {
          lotId: "lot-a",
          qualificationResult: "pass",
          isMandatory: true,
          lotAddressableValuePence: 50_000,
          candidates: [
            {
              candidateId: "cand-a1",
              hasPricing: true,
              hasLandedCost: true,
              passesNetGate: true,
              candidateValueShareBps: 10_000,
            },
          ],
        },
        {
          lotId: "lot-b",
          qualificationResult: "review",
          isMandatory: true,
          lotAddressableValuePence: null,
          candidates: [
            {
              candidateId: "cand-b1",
              hasPricing: true,
              hasLandedCost: true,
              passesNetGate: true,
              candidateValueShareBps: 10_000,
            },
          ],
        },
      ],
    });

    expect(missingValueOutput.netQualifiedValueCoverageBps).toBe(0);
    expect(missingValueOutput.netQualifiedValueCoverageBps).toBeLessThan(
      AUTO_QUALIFICATION_VALUE_FLOOR_BPS,
    );
  });

  it("TS-COV-006: produces a stable basis hash for canonical input and changes it when operator-relevant inputs change", () => {
    const canonicalInput: CoverageInput = {
      lots: [
        {
          lotId: "lot-b",
          qualificationResult: "review",
          isMandatory: false,
          lotAddressableValuePence: 25_000,
          candidates: [
            {
              candidateId: "cand-b2",
              hasPricing: false,
              hasLandedCost: false,
              passesNetGate: false,
              candidateValueShareBps: 4_000,
            },
            {
              candidateId: "cand-b1",
              hasPricing: true,
              hasLandedCost: false,
              passesNetGate: false,
              candidateValueShareBps: 6_000,
            },
          ],
        },
        {
          lotId: "lot-a",
          qualificationResult: "pass",
          isMandatory: true,
          lotAddressableValuePence: 75_000,
          candidates: [
            {
              candidateId: "cand-a1",
              hasPricing: true,
              hasLandedCost: true,
              passesNetGate: true,
              candidateValueShareBps: 10_000,
            },
          ],
        },
      ],
    };

    const equivalentInput: CoverageInput = {
      lots: [...canonicalInput.lots].reverse().map((lot) => ({
        ...lot,
        candidates: [...lot.candidates].reverse(),
      })),
    };

    const changedInput: CoverageInput = {
      lots: canonicalInput.lots.map((lot) =>
        lot.lotId === "lot-b"
          ? {
              ...lot,
              candidates: lot.candidates.map((candidate) =>
                candidate.candidateId === "cand-b1"
                  ? { ...candidate, hasLandedCost: true }
                  : candidate,
              ),
            }
          : lot,
      ),
    };

    const canonicalOutput = computeCoverageRollup(canonicalInput);
    const equivalentOutput = computeCoverageRollup(equivalentInput);
    const changedOutput = computeCoverageRollup(changedInput);

    expect(canonicalOutput.decisionBasisHash).toBe(equivalentOutput.decisionBasisHash);
    expect(changedOutput.decisionBasisHash).not.toBe(canonicalOutput.decisionBasisHash);
  });
});
