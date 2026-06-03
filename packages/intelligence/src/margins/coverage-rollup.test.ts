import { describe, expect, it } from "bun:test";

import {
  GOODS_LED_REVIEW_REASON_CODES,
  INSTALLATION_LABOUR_WARNING_CODES,
  type GoodsLedReviewContextPayload,
} from "../../../contracts-schema/src/index";
import {
  AUTO_QUALIFICATION_VALUE_FLOOR_BPS,
  computeCoverageRollup,
  describeCoverageThreshold,
  getCoverageDecisionSupport,
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
    expect(isAutoQualificationReady(output.netQualifiedValueCoverageBps)).toBe(false);
  });

  it("TS-COV-003: exposes one stable decision-support contract for threshold bands and the value-weighted gate", () => {
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
    expect(
      getCoverageDecisionSupport({
        netQualifiedCoverageBps: 6999,
        netQualifiedValueCoverageBps: 4999,
      }),
    ).toEqual({
      thresholdBand: {
        tone: "amber",
        label: "Partial coverage -- review recommended",
      },
      valueWeightedGateReady: false,
      autoQualificationFloorBps: AUTO_QUALIFICATION_VALUE_FLOOR_BPS,
      labourPermitCoverageReady: true,
      reviewRequired: true,
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

  it("TS-COV-006: returns deterministic coverage metrics without authoring a decision basis hash", () => {
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

    expect(canonicalOutput).toEqual(equivalentOutput);
    expect(changedOutput).not.toEqual(canonicalOutput);
    expect("decisionBasisHash" in canonicalOutput).toBe(false);
  });

  it("TS-COV-007: computes goods_value_share_bps cost-proxy arithmetic", () => {
    const output = computeCoverageRollup({
      lots: [],
      goodsLandedCostGbpPence: 1_000_000,
      installationLabourCostGbpPence: 240_000,
      permitTrafficMgmtCostGbpPence: 200_000,
      maintenanceAnnualisedOpexGbpPence: 240_000,
      maintenanceTotalVisibleTermOpexGbpPence: 720_000,
    });

    expect(output.goodsValueShareBps).toBe(6944);
  });

  it("TS-COV-010: excludes recurring maintenance opex from goods share denominator", () => {
    const output = computeCoverageRollup({
      lots: [],
      goodsLandedCostGbpPence: 1_800_000,
      installationLabourCostGbpPence: 450_000,
      permitTrafficMgmtCostGbpPence: 50_000,
      maintenanceAnnualisedOpexGbpPence: 240_000,
      maintenanceTotalVisibleTermOpexGbpPence: 720_000,
    });

    expect(output.goodsValueShareBps).toBe(7826);
  });

  it("TS-COV-008: permits/TM push goods share down through the all-in denominator", () => {
    const prePermitOutput = computeCoverageRollup({
      lots: [],
      goodsLandedCostGbpPence: 1_000_000,
      installationLabourCostGbpPence: 240_000,
      permitTrafficMgmtCostGbpPence: 0,
    });
    const allInOutput = computeCoverageRollup({
      lots: [],
      goodsLandedCostGbpPence: 1_000_000,
      installationLabourCostGbpPence: 240_000,
      permitTrafficMgmtCostGbpPence: 200_000,
    });
    const permitHeavyOutput = computeCoverageRollup({
      lots: [],
      goodsLandedCostGbpPence: 1_000_000,
      installationLabourCostGbpPence: 240_000,
      permitTrafficMgmtCostGbpPence: 540_000,
    });

    expect(prePermitOutput.goodsValueShareBps).toBe(8065);
    expect(allInOutput.goodsValueShareBps).toBe(6944);
    expect(permitHeavyOutput.goodsValueShareBps).toBeLessThan(6000);
  });

  it("TS-COV-009: adds labour/permit coverage metrics with conservative null handling", () => {
    const completeOutput = computeCoverageRollup({
      lots: [
        {
          lotId: "lot-a",
          qualificationResult: "pass",
          isMandatory: true,
          lotAddressableValuePence: 100_000,
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
      installationRequirementPresent: true,
      permitTrafficMgmtRequirementPresent: true,
      installationLabourCostGbpPence: 240_000,
      permitTrafficMgmtCostGbpPence: 200_000,
    });
    const missingLabourOutput = computeCoverageRollup({
      lots: [
        {
          lotId: "lot-a",
          qualificationResult: "pass",
          isMandatory: true,
          lotAddressableValuePence: 100_000,
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
      installationRequirementPresent: true,
      permitTrafficMgmtRequirementPresent: true,
      installationLabourCostGbpPence: null,
      permitTrafficMgmtCostGbpPence: 200_000,
    });
    const notApplicableOutput = computeCoverageRollup({
      lots: [],
      installationRequirementPresent: false,
      permitTrafficMgmtRequirementPresent: false,
    });

    expect(completeOutput.installationLabourCoverageBps).toBe(10000);
    expect(completeOutput.installationLabourValueCoverageBps).toBe(10000);
    expect(completeOutput.permitTrafficMgmtCoverageBps).toBe(10000);
    expect(completeOutput.permitTrafficMgmtValueCoverageBps).toBe(10000);
    expect(completeOutput.labourPermitCoverageReviewRequired).toBe(false);
    expect(getCoverageDecisionSupport(completeOutput).valueWeightedGateReady).toBe(
      true,
    );

    expect(missingLabourOutput.installationLabourCoverageBps).toBe(0);
    expect(missingLabourOutput.installationLabourValueCoverageBps).toBe(0);
    expect(missingLabourOutput.permitTrafficMgmtCoverageBps).toBe(10000);
    expect(missingLabourOutput.labourPermitCoverageReviewRequired).toBe(true);
    expect(getCoverageDecisionSupport(missingLabourOutput)).toMatchObject({
      labourPermitCoverageReady: false,
      valueWeightedGateReady: false,
      reviewRequired: true,
    });
    expect(GOODS_LED_REVIEW_REASON_CODES).toEqual([
      "near_floor_cost_proxy",
      "unresolved_permit_responsibility",
      "no_list_bespoke_flag",
      "labour_uncertainty",
    ]);
    expect(INSTALLATION_LABOUR_WARNING_CODES).toContain(
      "PERMIT_RESPONSIBILITY_UNKNOWN",
    );

    const reviewContext = {
      labourLow: null,
      labourBase: null,
      labourHigh: null,
      labourConfidenceBps: null,
      permitLow: 80_000,
      permitBase: 120_000,
      permitHigh: 200_000,
      permitResponsibility: "unknown",
      region: "north_west",
      installationScope: "festive-lighting-install-and-remove",
      equipmentAssumptions: ["operated_boom_mewp"],
      warningCodes: [
        "ACCESS_AT_HEIGHT_ASSUMED_MEWP",
        "PERMIT_RESPONSIBILITY_UNKNOWN",
      ],
      goodsShareBasis: "cost_proxy",
      goodsShareFloorBps: AUTO_QUALIFICATION_VALUE_FLOOR_BPS,
      decisionRoute: "review",
      labourCoverageBps: missingLabourOutput.installationLabourCoverageBps,
      reviewReasons: [
        "near_floor_cost_proxy",
        "unresolved_permit_responsibility",
        "no_list_bespoke_flag",
        "labour_uncertainty",
      ],
    } satisfies GoodsLedReviewContextPayload;

    expect(reviewContext).toMatchObject({
      labourCoverageBps: 0,
      decisionRoute: "review",
      goodsShareBasis: "cost_proxy",
      goodsShareFloorBps: AUTO_QUALIFICATION_VALUE_FLOOR_BPS,
    });

    expect(notApplicableOutput.installationLabourCoverageBps).toBeNull();
    expect(notApplicableOutput.installationLabourValueCoverageBps).toBeNull();
    expect(notApplicableOutput.permitTrafficMgmtCoverageBps).toBeNull();
    expect(notApplicableOutput.permitTrafficMgmtValueCoverageBps).toBeNull();
    expect(notApplicableOutput.labourPermitCoverageReviewRequired).toBe(false);
  });
});
