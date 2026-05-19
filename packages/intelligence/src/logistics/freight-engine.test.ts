import { describe, expect, it } from "bun:test";

import type { ModeSelectionFacts, ShippingMode } from "./freight-types";
import { selectMode } from "./mode-selector";

// [TEST_START] L5 freight engine scaffold.
// L5-T-freight_engine-001,002 land in l5-p3-logistics-types-mode-selector.
// L5-T-freight_engine-003,004,005,006,007,008 land in later Phase 3 beads.
// L5-T-freight_engine-012,015,016 are daemon/source-health integration tests.

describe("freight mode selector", () => {
  it("L5-T-freight_engine-001 selects modes from weight volume value urgency product locks and packaging facts", () => {
    const cases: Array<{
      name: string;
      facts: ModeSelectionFacts;
      expectedMode: ShippingMode;
      expectedFallback: boolean;
    }> = [
      {
        name: "critical small parcel uses express when weight and volume permit",
        facts: baseFacts({
          weightKg: 12,
          volumeM3: 0.2,
          urgency: "critical",
        }),
        expectedMode: "air_express",
        expectedFallback: false,
      },
      {
        name: "expedited medium freight uses general air",
        facts: baseFacts({
          volumeM3: 3,
          urgency: "expedited",
        }),
        expectedMode: "air_general",
        expectedFallback: false,
      },
      {
        name: "standard medium freight uses sea LCL",
        facts: baseFacts({
          volumeM3: 8,
        }),
        expectedMode: "sea_lcl",
        expectedFallback: false,
      },
      {
        name: "large freight uses sea FCL",
        facts: baseFacts({
          volumeM3: 18,
        }),
        expectedMode: "sea_fcl",
        expectedFallback: false,
      },
      {
        name: "high-value standard freight uses multimodal risk control",
        facts: baseFacts({
          valueGbpPence: 7_500_000,
          volumeM3: 6,
        }),
        expectedMode: "multimodal",
        expectedFallback: false,
      },
      {
        name: "care-sensitive product type affects critical handling",
        facts: baseFacts({
          productType: "medical diagnostic analyser",
          urgency: "critical",
          volumeM3: 1.2,
        }),
        expectedMode: "air_general",
        expectedFallback: false,
      },
      {
        name: "unknown packaging uses a conservative highest-cost boundary",
        facts: baseFacts({
          packagingState: "unknown",
          weightKg: 10,
          volumeM3: 0.2,
          urgency: "standard",
        }),
        expectedMode: "air_express",
        expectedFallback: true,
      },
    ];

    for (const testCase of cases) {
      const selection = selectMode(testCase.facts);
      expect(selection.mode, testCase.name).toBe(testCase.expectedMode);
      expect(selection.fallbackBoundaryUsed, testCase.name).toBe(
        testCase.expectedFallback,
      );
      expect(selection.reason, testCase.name).toContain(
        testCase.facts.productType,
      );
      expect(selection.confidenceBps, testCase.name).toBeGreaterThan(0);
    }
  });

  it("L5-T-freight_engine-002 hard locks override cheaper infeasible modes", () => {
    const cases: Array<{
      name: string;
      facts: ModeSelectionFacts;
      expectedMode: ShippingMode;
      expectedLock: string;
      forbiddenMode: ShippingMode;
    }> = [
      {
        name: "hazmat excludes air express",
        facts: baseFacts({
          hazmat: true,
          weightKg: 5,
          volumeM3: 0.2,
          urgency: "critical",
        }),
        expectedMode: "sea_lcl",
        expectedLock: "hazmat",
        forbiddenMode: "air_express",
      },
      {
        name: "loose lithium batteries exclude air",
        facts: baseFacts({
          batteries: "loose",
          weightKg: 5,
          volumeM3: 0.2,
          urgency: "critical",
        }),
        expectedMode: "sea_lcl",
        expectedLock: "battery",
        forbiddenMode: "air_express",
      },
      {
        name: "reefer container lock chooses FCL for large volume",
        facts: baseFacts({
          reefer: true,
          volumeM3: 18,
          urgency: "standard",
        }),
        expectedMode: "sea_fcl",
        expectedLock: "reefer",
        forbiddenMode: "sea_lcl",
      },
      {
        name: "oversized excludes express while preserving urgency",
        facts: baseFacts({
          oversized: true,
          weightKg: 20,
          volumeM3: 0.4,
          urgency: "critical",
        }),
        expectedMode: "air_general",
        expectedLock: "oversized",
        forbiddenMode: "air_express",
      },
      {
        name: "critical fragile freight uses controlled general air",
        facts: baseFacts({
          fragile: true,
          weightKg: 6,
          volumeM3: 0.2,
          urgency: "critical",
        }),
        expectedMode: "air_general",
        expectedLock: "fragile",
        forbiddenMode: "air_express",
      },
    ];

    for (const testCase of cases) {
      const selection = selectMode(testCase.facts);
      expect(selection.mode, testCase.name).toBe(testCase.expectedMode);
      expect(selection.mode, testCase.name).not.toBe(testCase.forbiddenMode);
      expect(
        selection.hardLocksTriggered.map((lock) => lock.kind),
        testCase.name,
      ).toContain(testCase.expectedLock);
    }
  });
});

function baseFacts(
  overrides: Partial<ModeSelectionFacts> = {},
): ModeSelectionFacts {
  return {
    weightKg: 250,
    volumeM3: 4,
    valueGbpPence: 250_000,
    urgency: "standard",
    productType: "paperboard cartons",
    hazmat: false,
    oversized: false,
    fragile: false,
    reefer: false,
    batteries: "none",
    packagingState: "palletized",
    ...overrides,
  };
}
