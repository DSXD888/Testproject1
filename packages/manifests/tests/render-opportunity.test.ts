import { describe, expect, test } from "bun:test";

import {
  renderOpportunityManifest,
  type ManifestInput
} from "../render-opportunity.ts";

const BASE_INPUT: ManifestInput = {
  process: {
    id: "process-1",
    canonicalProcessId: "ocds-123",
    title: "Nitrile gloves and surgical masks",
    buyerName: "NHS Test Buyer",
    latestPublishedAt: "2026-03-28T08:00:00Z",
    latestUpdatedAt: "2026-03-28T09:00:00Z",
    latestDeadlineAt: "2026-04-10T12:00:00Z",
    accessRoute: "FTS",
    estimatedValueGbpPence: 1234567
  },
  notices: [
    {
      id: "notice-1",
      title: "PPE framework refresh",
      buyerName: "NHS Test Buyer",
      publishedAt: "2026-03-28T08:00:00Z",
      deadlineAt: "2026-04-10T12:00:00Z",
      accessRoute: "FTS"
    }
  ],
  lots: [
    {
      id: "lot-1",
      lotNumber: 1,
      title: "Gloves",
      estimatedValuePence: 900000,
      qualificationStatus: "qualified",
      addressableValuePence: 750000,
      goodsClassification: "goods"
    },
    {
      id: "lot-2",
      lotNumber: 2,
      title: "Masks",
      estimatedValuePence: 334567,
      qualificationStatus: "review",
      addressableValuePence: 250000,
      goodsClassification: "mixed"
    }
  ],
  candidates: [
    {
      id: "candidate-1",
      lotId: "lot-1",
      name: "Disposable nitrile exam gloves",
      archetype: "exact-match",
      identityConfidenceBps: 9200,
      quoteabilityScoreBps: 8100
    }
  ],
  quotes: [
    {
      id: "quote-1",
      candidateId: "candidate-1",
      supplierName: "MedicTrade",
      unitPricePence: 420
    }
  ],
  pricingAssessments: [
    {
      id: "assessment-1",
      candidateId: "candidate-1",
      source: "eBay",
      priceSignalRole: "primary",
      matchConfidenceBps: 9100,
      usabilityConfidenceBps: 8800
    }
  ],
  landedCosts: [
    {
      id: "landed-1",
      candidateId: "candidate-1",
      incoterm: "DDP",
      freightPence: 1200,
      dutyPence: 800,
      totalPence: 6200
    }
  ],
  pipelineAssessment: {
    decisionBasisHash:
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    generatedAt: "2026-03-28T09:30:00Z",
    recommendation: {
      disposition: "manual-review",
      reason: "One lot still needs buyer clarification."
    },
    grossMargin: {
      outcome: "review",
      basisPoints: 8750
    },
    netMarginBand: {
      lowBps: 6200,
      baseBps: 7100,
      highBps: 7900
    },
    riskFlags: [
      {
        label: "Buyer requests alternate packaging proof",
        detail: "Awaiting confirmation for lot 2",
        severity: "blocker"
      }
    ]
  },
  coverageMetrics: {
    pricedCoverageBps: 8750,
    landedCostCoverageBps: 8200,
    netQualifiedCoverageBps: 7100,
    unresolvedMandatoryLots: 1
  }
};

function cloneInput(): ManifestInput {
  return structuredClone(BASE_INPUT);
}

describe("renderOpportunityManifest", () => {
  test("TS-MAN-001 returns all required output fields in the happy path", () => {
    const output = renderOpportunityManifest(cloneInput());

    expect(output.markdown.length).toBeGreaterThan(0);
    expect(output.basisHash).toBe(
      BASE_INPUT.pipelineAssessment.decisionBasisHash
    );
    expect(output.sections.length).toBe(8);
    expect(output.blockerCount).toBe(1);
    expect(output.generatedAt).toBe("2026-03-28T09:30:00Z");
  });

  test("TS-MAN-002 passes through the daemon-authored basis hash exactly", () => {
    const input = cloneInput();
    input.pipelineAssessment.decisionBasisHash =
      "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

    const output = renderOpportunityManifest(input);

    expect(output.basisHash).toBe(input.pipelineAssessment.decisionBasisHash);
  });

  test("TS-MAN-003 keeps the canonical section order and stable ids", () => {
    const output = renderOpportunityManifest(cloneInput());

    expect(output.sections).toEqual([
      "summary",
      "lot-analysis",
      "product-candidates",
      "pricing-evidence",
      "margin-estimate",
      "coverage-metrics",
      "risk-flags",
      "recommendation"
    ]);

    const headingOrder = [
      "## Summary",
      "## Lot Analysis",
      "## Product Candidates",
      "## Pricing Evidence",
      "## Margin Estimate",
      "## Coverage Metrics",
      "## Risk Flags",
      "## Recommendation"
    ];
    let previousIndex = -1;
    for (const heading of headingOrder) {
      const index = output.markdown.indexOf(heading);
      expect(index).toBeGreaterThan(previousIndex);
      previousIndex = index;
    }
  });

  test("TS-MAN-004 keeps empty-data sections explicit instead of omitting them", () => {
    const input = cloneInput();
    input.candidates = [];
    input.quotes = [];
    input.pricingAssessments = [];
    input.landedCosts = [];

    const output = renderOpportunityManifest(input);

    expect(output.markdown).toContain("## Product Candidates");
    expect(output.markdown).toContain("No product candidates identified yet.");
    expect(output.markdown).toContain("## Pricing Evidence");
    expect(output.markdown).toContain("No pricing evidence is available yet.");
    expect(output.markdown).toContain("## Margin Estimate");
    expect(output.markdown).toContain("No landed-cost estimates available yet.");
  });

  test("TS-MAN-005 formats money and percentages from integer inputs and keeps blockers prominent", () => {
    const output = renderOpportunityManifest(cloneInput());

    expect(output.markdown).toContain("£12,345.67");
    expect(output.markdown).toContain("87.50%");
    expect(output.markdown).toContain("**BLOCKER**");
    expect(output.markdown).toContain("high");
    expect(output.markdown).not.toContain("0.875");
    expect(output.markdown).not.toContain("0.92");
  });
});
