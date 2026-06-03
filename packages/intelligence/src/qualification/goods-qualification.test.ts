import { describe, expect, it } from "bun:test";

import {
  classifyAccessRoute,
  classifyLegalCategory,
  evaluateExpiry,
  qualifyGoodsOpportunity,
} from "./goods-qualification";

describe("goods qualification", () => {
  it("TS-QUAL-001: classifies CPV families into goods, services, works, mixed, and unknown deterministically", () => {
    expect(classifyLegalCategory(["33100000-1"])).toBe("goods");
    expect(classifyLegalCategory(["50750000-7"])).toBe("services");
    expect(classifyLegalCategory(["45233120-6"])).toBe("works");
    expect(classifyLegalCategory(["33100000-1", "50750000-7"])).toBe("mixed");
    expect(classifyLegalCategory(["invalid"])).toBe("unknown");
  });

  it("TS-QUAL-002: treats expired ordinary notices as rejected while keeping dynamic markets eligible", () => {
    const now = "2026-03-28T00:00:00.000Z";

    const ordinary = qualifyGoodsOpportunity({
      cpvCodes: ["33100000-1"],
      tenderDeadline: "2026-04-05T00:00:00.000Z",
      now,
      procedureType: "open procedure",
      qualificationBasis: "notice_only",
      totalValuePence: 100_000,
    });
    const dps = qualifyGoodsOpportunity({
      cpvCodes: ["33100000-1"],
      tenderDeadline: "2026-04-05T00:00:00.000Z",
      now,
      procedureType: "dynamic market",
      qualificationBasis: "notice_only",
      totalValuePence: 100_000,
    });

    expect(ordinary.qualificationResult).toBe("reject");
    expect(dps.qualificationResult).toBe("pass");
    expect(dps.accessRoute).toBe("dynamic_market");
  });

  it("TS-QUAL-003a: passes separable mixed at or above the goods value-share floor", () => {
    const result = qualifyGoodsOpportunity({
      cpvCodes: ["33100000-1", "50750000-7"],
      tenderDeadline: "2026-06-30T00:00:00.000Z",
      now: "2026-03-28T00:00:00.000Z",
      accessRoute: "open_tender",
      qualificationBasis: "lot_level",
      totalValuePence: 900_000,
      goodsAddressableValuePence: 540_000,
      goodsSeparability: true,
      goodsDensitySignals: {
        physicalGoodsMentions: 3,
        quantityMentions: 2,
      },
    });

    expect(result.legalCategory).toBe("mixed");
    expect(result.addressableValuePence).toBe(540_000);
    expect(result.qualificationResult).toBe("pass");
  });

  it("TS-QUAL-003b: reviews separable mixed below the goods value-share floor", () => {
    const result = qualifyGoodsOpportunity({
      cpvCodes: ["33100000-1", "50750000-7"],
      tenderDeadline: "2026-06-30T00:00:00.000Z",
      now: "2026-03-28T00:00:00.000Z",
      accessRoute: "open_tender",
      qualificationBasis: "lot_level",
      totalValuePence: 900_000,
      goodsAddressableValuePence: 360_000,
      goodsSeparability: true,
      goodsDensitySignals: {
        physicalGoodsMentions: 3,
        quantityMentions: 2,
      },
    });

    expect(result.legalCategory).toBe("mixed");
    expect(result.addressableValuePence).toBe(360_000);
    expect(result.qualificationResult).toBe("review");
  });

  it("TS-QUAL-004: preserves reviewability and rationale for low-confidence cases", () => {
    const result = qualifyGoodsOpportunity({
      cpvCodes: ["33100000-1"],
      tenderDeadline: "2026-06-30T00:00:00.000Z",
      now: "2026-03-28T00:00:00.000Z",
      accessRoute: "open_tender",
      qualificationBasis: "notice_only",
      totalValuePence: 200_000,
      goodsConfidenceBps: 4500,
      rationale: ["Initial deterministic CPV match indicates a medical-goods family."],
    });

    expect(result.qualificationResult).toBe("review");
    expect(result.goodsQualified).toBe(true);
    expect(result.rationale).toContain(
      "Initial deterministic CPV match indicates a medical-goods family.",
    );
    expect(result.rationale.some((entry) => entry.includes("autonomous threshold"))).toBe(
      true,
    );
  });

  it("TS-QUAL-005: preserves access-route edge cases without UI-side heuristics", () => {
    expect(classifyAccessRoute({ procedureType: "restricted procedure" })).toBe(
      "restricted_tender",
    );
    expect(classifyAccessRoute({ procedureType: "framework call off" })).toBe(
      "framework_call_off",
    );
    expect(classifyAccessRoute({ procedureType: "direct award" })).toBe("direct_award");
    expect(
      evaluateExpiry({
        accessRoute: "dynamic_market",
        now: "2026-03-28T00:00:00.000Z",
        tenderDeadline: "2026-03-29T00:00:00.000Z",
      }),
    ).toBe("pass");
  });

  it("TS-QUAL-006: provisionally admits credible goods-bearing mixed and works notices", () => {
    const mixed = qualifyGoodsOpportunity({
      cpvCodes: ["33100000-1", "50750000-7"],
      tenderDeadline: "2026-06-30T00:00:00.000Z",
      now: "2026-03-28T00:00:00.000Z",
      accessRoute: "open_tender",
      qualificationBasis: "notice_only",
      totalValuePence: 900_000,
      goodsDensitySignals: {
        serviceMentions: 2,
      },
      goodsConfidenceBps: 6500,
    });
    const works = qualifyGoodsOpportunity({
      cpvCodes: ["45233120-6"],
      tenderDeadline: "2026-06-30T00:00:00.000Z",
      now: "2026-03-28T00:00:00.000Z",
      accessRoute: "open_tender",
      qualificationBasis: "notice_only",
      totalValuePence: 900_000,
      goodsSeparability: true,
      goodsDensitySignals: {
        physicalGoodsMentions: 1,
      },
      goodsConfidenceBps: 6500,
    });
    const serviceOnly = qualifyGoodsOpportunity({
      cpvCodes: ["50750000-7"],
      tenderDeadline: "2026-06-30T00:00:00.000Z",
      now: "2026-03-28T00:00:00.000Z",
      accessRoute: "open_tender",
      qualificationBasis: "notice_only",
      totalValuePence: 900_000,
      goodsConfidenceBps: 6500,
    });

    expect(mixed.legalCategory).toBe("mixed");
    expect(mixed.goodsDensityScore).toBeCloseTo(0.44);
    expect(mixed.qualificationResult).toBe("review");
    expect(works.legalCategory).toBe("works");
    expect(works.goodsDensityScore).toBeCloseTo(0.35);
    expect(works.qualificationResult).toBe("review");
    expect(serviceOnly.legalCategory).toBe("services");
    expect(serviceOnly.qualificationResult).toBe("reject");
  });

  it("TS-QUAL-008: prefers contract-value share and never passes separable mixed on density alone", () => {
    const valueSharePass = qualifyGoodsOpportunity({
      cpvCodes: ["33100000-1", "50750000-7"],
      tenderDeadline: "2026-06-30T00:00:00.000Z",
      now: "2026-03-28T00:00:00.000Z",
      accessRoute: "open_tender",
      qualificationBasis: "lot_level",
      totalValuePence: 900_000,
      goodsAddressableValuePence: 540_000,
      goodsSeparability: true,
      goodsDensitySignals: {
        serviceMentions: 1,
      },
      goodsConfidenceBps: 6500,
    });
    const missingNumerator = qualifyGoodsOpportunity({
      cpvCodes: ["33100000-1", "50750000-7"],
      tenderDeadline: "2026-06-30T00:00:00.000Z",
      now: "2026-03-28T00:00:00.000Z",
      accessRoute: "open_tender",
      qualificationBasis: "lot_level",
      totalValuePence: 900_000,
      goodsSeparability: true,
      goodsDensitySignals: {
        physicalGoodsMentions: 6,
        quantityMentions: 4,
      },
      goodsConfidenceBps: 9000,
    });

    expect(valueSharePass.addressableValuePence).toBe(540_000);
    expect(valueSharePass.qualificationResult).toBe("pass");
    expect(missingNumerator.addressableValuePence).toBeNull();
    expect(missingNumerator.qualificationResult).toBe("review");
  });
});
