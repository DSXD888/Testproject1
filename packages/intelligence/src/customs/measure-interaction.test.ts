import { describe, expect, it } from "bun:test";

import { EXPORTER_UNRESOLVED, resolveExporter } from "./exporter-resolution";
import {
  MEASURE_INTERACTION_UNKNOWN,
  SUPPLEMENTARY_UNIT_MISSING,
  scanMeasures,
  type ActiveMeasure,
} from "./measure-scanner";

const repoRoot = new URL("../../../../", import.meta.url);

const redactionDesignUrl = new URL(
  "docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-redaction-sentinel-design.md",
  repoRoot,
);
const sentinelFixtureUrl = new URL(
  "tests/fixtures/l5/redaction/sentinels.yaml",
  repoRoot,
);
const sqliteMigrationUrl = new URL(
  "migrations/0001_sqlite_state_initial.sql",
  repoRoot,
);
const productCommodityCodesMigrationUrl = new URL(
  "migrations/0013_l5_product_commodity_codes.sql",
  repoRoot,
);

const automationSensitiveFields = [
  "exporter_rate_basis",
  "exporter_identity_confidence_bps",
  "origin_evidence_state",
  "trade_remedy_cases_json",
  "duty_breakdown_json",
  "measure_interaction_json",
  "confidence_breakdown.evidence_leverage",
  "product_commodity_codes.evidence_json",
] as const;

const displayFields = [
  "declarable_commodity_code_10",
  "net_margin_low_bps",
  "net_margin_base_bps",
  "net_margin_high_bps",
  "decision_grade",
  "warnings",
  "blockers",
  "cbam_exposure_flag",
] as const;

describe("measure scanner interaction families", () => {
  it("L5-T-measure_interaction-001 keeps additive steel duty components separate", () => {
    const result = scanMeasures({
      uk10: "7308909800",
      measures: [
        adValoremMeasure("mfn-steel", "MFN", 200),
        adValoremMeasure("ad-steel-cn", "AD", 2_100),
        adValoremMeasure("cvd-steel-cn", "CVD", 400),
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: exporterResolutionFixture(),
    });

    expect(result.blockers).toEqual([]);
    expect(result.selectedDutyPence).toBe(270_000);
    expect(result.perMeasureClassification.map((entry) => entry.family)).toEqual([
      "additive",
      "additive",
      "additive",
    ]);
    expect(result.dutyBreakdownJson).toMatchObject({
      kind: "duty_breakdown",
      components: [
        { measureId: "mfn-steel", family: "additive", amountPence: 20_000 },
        { measureId: "ad-steel-cn", family: "additive", amountPence: 210_000 },
        { measureId: "cvd-steel-cn", family: "additive", amountPence: 40_000 },
      ],
    });
  });

  it("L5-T-measure_interaction-002 applies substitutive greater-of steel arithmetic", () => {
    const result = scanMeasures({
      uk10: "7308909800",
      measures: [
        adValoremMeasure("safeguard-steel", "safeguard", 2_500),
        adValoremMeasure("ad-steel-cn", "AD", 2_110),
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: exporterResolutionFixture(),
    });

    expect(result.selectedDutyPence).toBe(250_000);
    expect(result.selectedDutyPence).not.toBe(461_000);
    expect(result.perMeasureClassification.map((entry) => entry.family)).toEqual([
      "substitutive",
      "substitutive",
    ]);
    expect(result.dutyBreakdownJson).toMatchObject({
      substitutiveGroups: [
        {
          rule: "greater_of",
          selectedMeasureId: "safeguard-steel",
          components: [
            { measureId: "safeguard-steel", amountPence: 250_000, selected: true },
            { measureId: "ad-steel-cn", amountPence: 211_000, selected: false },
          ],
        },
      ],
    });
  });

  it("L5-T-measure_interaction-003 uses out-of-quota rates without allocation proof", () => {
    const result = scanMeasures({
      uk10: "7318158898",
      measures: [
        {
          measureType: "TRQ",
          id: "quota-fasteners",
          dutyExpression: { kind: "ad_valorem", rateBps: 0 },
          conditions: {
            quotaCode: "UKQ-FASTENERS",
            quotaProofAvailable: false,
            inQuotaDutyExpression: { kind: "ad_valorem", rateBps: 0 },
            outOfQuotaDutyExpression: { kind: "ad_valorem", rateBps: 1_200 },
            evidenceRequired: ["quota_allocation_proof"],
          },
        },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: exporterResolutionFixture(),
    });

    expect(result.selectedDutyPence).toBe(120_000);
    expect(result.perMeasureClassification[0]?.family).toBe("conditional");
    expect(result.measureInteractionJson).toMatchObject({
      conditions: [
        {
          measureId: "quota-fasteners",
          quotaCode: "UKQ-FASTENERS",
          selectedBranch: "out_of_quota",
          evidenceRequired: ["quota_allocation_proof"],
        },
      ],
    });
  });

  it("L5-T-measure_interaction-004 blocks auto-decision for unknown interaction rules", () => {
    const result = scanMeasures({
      uk10: "7308909800",
      measures: [
        {
          id: "unmapped-remedy-stack",
          measureType: "unmapped_AD_CVD_safeguard_stack",
          dutyExpression: { kind: "unknown" },
        },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: exporterResolutionFixture(),
    });

    expect(result.selectedDutyPence).toBeNull();
    expect(result.blockers).toContain(MEASURE_INTERACTION_UNKNOWN);
    expect(result.perMeasureClassification[0]?.family).toBe("unknown");
  });

  it("L5-T-measure_interaction-005 keeps additional-code prerequisites separate from eligibility", () => {
    const result = scanMeasures({
      uk10: "4819100000",
      measures: [
        {
          id: "de-6-17-paperboard",
          measureType: "additional_code_relief",
          dutyExpression: { kind: "ad_valorem", rateBps: 0 },
          conditions: {
            additionalCode: "DE 6/17",
            additionalCodeSatisfiability: "unknown",
            evidenceRequired: ["vat_relief_document", "commodity_prerequisite"],
            applyWhenAdditionalCodeSatisfied: { kind: "ad_valorem", rateBps: 0 },
            rateWithoutAdditionalCode: { kind: "ad_valorem", rateBps: 2_000 },
          },
        },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: exporterResolutionFixture(),
    });

    expect(result.selectedDutyPence).toBe(200_000);
    expect(result.additionalCodesRequirementsJson).toEqual([
      {
        code: "DE 6/17",
        satisfiability: "unknown",
        evidenceRequired: ["vat_relief_document", "commodity_prerequisite"],
      },
    ]);
    expect(result.measureInteractionJson).toMatchObject({
      conditions: [
        {
          measureId: "de-6-17-paperboard",
          additionalCode: "DE 6/17",
          selectedBranch: "without_additional_code",
        },
      ],
    });
  });

  it("L5-T-measure_interaction-006 labels every active measure before computation", () => {
    const result = scanMeasures({
      uk10: "7308909800",
      measures: [
        adValoremMeasure("mfn", "MFN", 200),
        adValoremMeasure("safeguard", "safeguard", 2_500),
        adValoremMeasure("ad", "AD", 2_110),
        {
          id: "quota",
          measureType: "quota",
          dutyExpression: { kind: "ad_valorem", rateBps: 1_200 },
          conditions: { quotaCode: "UKQ", quotaProofAvailable: false },
        },
        { id: "licence", measureType: "documentary", dutyExpression: { kind: "ad_valorem", rateBps: 0 } },
        { id: "unknown", measureType: "unknown_stack", dutyExpression: { kind: "unknown" } },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: exporterResolutionFixture(),
    });

    expect(result.perMeasureClassification.map((entry) => entry.family)).toEqual([
      "additive",
      "substitutive",
      "substitutive",
      "conditional",
      "documentary_only",
      "unknown",
    ]);
    expect(result.blockers).toContain(MEASURE_INTERACTION_UNKNOWN);
  });

  it("L5-T-measure_interaction-007 preserves compound duties without flattening to dutyPct", () => {
    const result = scanMeasures({
      uk10: "6403999600",
      measures: [
        {
          id: "compound-footwear",
          measureType: "MFN",
          dutyExpression: {
            kind: "compound",
            components: [
              { kind: "ad_valorem", rateBps: 500 },
              { kind: "specific", unit: "item", amountPence: 20 },
            ],
          },
        },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: { item: 100 },
      exporterResolution: exporterResolutionFixture(),
    });

    expect(result.selectedDutyPence).toBe(52_000);
    expect(result.dutyBreakdownJson).toMatchObject({
      components: [
        {
          measureId: "compound-footwear",
          expression: {
            kind: "compound",
            components: [
              { kind: "ad_valorem", amountPence: 50_000 },
              { kind: "specific", unit: "item", quantity: 100, amountPence: 2_000 },
            ],
          },
        },
      ],
    });
    expect(JSON.stringify(result.dutyBreakdownJson)).not.toContain("dutyPct");
  });

  it("L5-T-measure_interaction-008 abstains when required supplementary units are missing", () => {
    const result = scanMeasures({
      uk10: "2202100000",
      measures: [
        {
          id: "specific-litres",
          measureType: "specific_duty",
          dutyExpression: { kind: "specific", unit: "litre", amountPence: 100 },
        },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: exporterResolutionFixture(),
    });

    expect(result.selectedDutyPence).toBeNull();
    expect(result.blockers).toContain(SUPPLEMENTARY_UNIT_MISSING);
    expect(result.supplementaryUnitsRequiredJson).toEqual(["litre"]);
    expect(result.dutyBreakdownJson).toMatchObject({
      components: [{ measureId: "specific-litres", amountPence: null }],
    });
  });

  it("L5-T-measure_interaction-009 preserves GIR 3 set candidates for manual classification", () => {
    const result = scanMeasures({
      uk10: "9503007000",
      measures: [adValoremMeasure("mfn-set", "MFN", 470)],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: exporterResolutionFixture(),
      classificationFlags: { gir3Candidate: true },
    });

    expect(result.selectedDutyPence).toBeNull();
    expect(result.blockers).toContain("gir_3_manual_classification_required");
    expect(result.measureInteractionJson).toMatchObject({
      gir3ManualClassificationRequired: true,
    });
  });

  it("L5-T-measure_interaction-010 refuses pieces-to-kg conversion without net-weight evidence", () => {
    const result = scanMeasures({
      uk10: "7318158898",
      measures: [
        {
          id: "specific-kg-net",
          measureType: "specific_duty",
          dutyExpression: { kind: "specific", unit: "kg", amountPence: 80 },
        },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: { item: 5_000 },
      exporterResolution: exporterResolutionFixture(),
    });

    expect(result.selectedDutyPence).toBeNull();
    expect(result.blockers).toContain(SUPPLEMENTARY_UNIT_MISSING);
    expect(result.supplementaryUnitsRequiredJson).toEqual(["kg_net"]);
  });
});

describe("measure interaction redaction sentinels", () => {
  it("L5-T-measure_interaction-011 defaults active trade remedies to residual when exporter proof is absent", () => {
    const result = resolveExporter({
      commodityCode: "7604210000",
      countryOfOrigin: "CN",
      customsValueGbpPence: 1_000_000,
      supplierQuotes: [],
      measuresAtCommodity: [aluminiumExtrusionMeasure()],
    });

    expect(result.exporterRateBasis).toBe("residual_default");
    expect(result.errorCode).toBe(EXPORTER_UNRESOLVED);
    expect(result.exporterIdentityConfidenceBps).toBeGreaterThanOrEqual(0);
    expect(result.exporterIdentityConfidenceBps).toBeLessThanOrEqual(10_000);
    expect(result.selectedRatePctBps).toBe(3510);
  });

  it("L5-T-measure_interaction-012 applies aluminium residual duty and records named-exporter delta", () => {
    const result = resolveExporter({
      commodityCode: "7604210000",
      countryOfOrigin: "CN",
      customsValueGbpPence: 1_000_000,
      supplierQuotes: [],
      measuresAtCommodity: [aluminiumExtrusionMeasure()],
    });

    expect(result.exporterRateBasis).toBe("residual_default");
    expect(result.selectedRatePctBps).toBe(3510);
    expect(result.appliedTradeRemedyDutyGbpPence).toBe(351_000);
    expect(result.unlockableMarginBps).toBe(3510);
    expect(result.evidenceLeverage[0]).toMatchObject({
      kind: "named_exporter_proof",
      unlockableMarginBps: 3510,
    });
  });

  it("L5-T-measure_interaction-013 rejects distributor marketplace and factory-direct names as named-exporter proof", () => {
    const weakClaims = [
      {
        supplierName: "Foshan Nanhai Aluminium Co Ltd",
        supplierRole: "distributor" as const,
      },
      {
        marketplaceSellerName: "Foshan Nanhai Aluminium Co Ltd",
        supplierRole: "marketplace_seller" as const,
      },
      {
        supplierName: "Factory Direct Aluminium Store",
        supplierRole: "unknown" as const,
        factoryDirectClaim: true,
      },
    ];

    for (const quote of weakClaims) {
      const result = resolveExporter({
        commodityCode: "7604210000",
        countryOfOrigin: "CN",
        customsValueGbpPence: 1_000_000,
        supplierQuotes: [quote],
        measuresAtCommodity: [aluminiumExtrusionMeasure()],
      });

      expect(result.exporterRateBasis).toBe("residual_default");
      expect(result.errorCode).toBe(EXPORTER_UNRESOLVED);
      expect(result.selectedRatePctBps).toBe(3510);
      expect(result.exporterIdentityConfidenceBps).toBeLessThanOrEqual(2500);
    }
  });

  it("L5-T-measure_interaction-020 skeleton maps viewer and operator sentinel boundaries", async () => {
    const [design, fixture] = await Promise.all([
      readText(redactionDesignUrl),
      readText(sentinelFixtureUrl),
    ]);

    expect(design).toContain("Phase 6 implements the field-level projection boundary");
    expect(design).toContain("table-level RLS as sufficient evidence");
    expect(fixture).toContain("automation_sensitive_fields:");
    expect(fixture).toContain("local_only_sensitive:");

    for (const field of automationSensitiveFields) {
      expect(design).toContain(field);
      expect(fixture).toContain(field);
      expect(fixtureFieldBlock(fixture, field)).toContain("viewer_visible: false");
      expect(fixtureFieldBlock(fixture, field)).toContain("operator_visible: true");
    }

    for (const field of displayFields) {
      expect(design).toContain(field);
      expect(fixtureFieldBlock(fixture, field)).toContain("viewer_visible: true");
      expect(fixtureFieldBlock(fixture, field)).toContain("operator_visible: true");
    }
  });

  it("L5-T-measure_interaction-020 skeleton ties sentinels to existing L0/L5 columns without adding landed-cost DDL", async () => {
    const [sqliteMigration, productCommodityCodesMigration, design] =
      await Promise.all([
        readText(sqliteMigrationUrl),
        readText(productCommodityCodesMigrationUrl),
        readText(redactionDesignUrl),
      ]);

    for (const field of automationSensitiveFields.slice(0, 7)) {
      const column = field.split(".")[0];
      expect(sqliteMigration).toContain(column);
    }

    expect(productCommodityCodesMigration).toContain("product_commodity_codes");
    expect(productCommodityCodesMigration).toContain("evidence_json");
    expect(design).toContain("L5 does not add columns to `landed_cost_estimates`");
    expect(design).toContain("product_commodity_codes.evidence_json");
    expect(design).toContain("0004_rls_policies.sql");
    expect(design).toContain("0005_sync_rpcs.sql");
  });
});

async function readText(url: URL): Promise<string> {
  return await Bun.file(url).text();
}

function fixtureFieldBlock(fixture: string, field: string): string {
  const escaped = escapeRegExp(field);
  const pattern = new RegExp(
    `(?:^|\\n)\\s{2}(?:"${escaped}"|${escaped}):\\n(?<block>(?:\\s{4}.+\\n?)+)`,
  );
  const match = fixture.match(pattern);
  expect(match?.groups?.block, `missing sentinel block for ${field}`).toBeTruthy();
  return match?.groups?.block ?? "";
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function aluminiumExtrusionMeasure() {
  return {
    id: "uk-ad-aluminium-extrusions-cn-residual",
    kind: "anti_dumping" as const,
    active: true,
    countryOfOrigin: "CN",
    residualRatePctBps: 3510,
    namedExporterRates: [
      {
        exporterName: "Foshan Nanhai Aluminium Co Ltd",
        manufacturerName: "Foshan Nanhai Aluminium Co Ltd",
        additionalCode: "C999",
        ratePctBps: 0,
      },
    ],
    annexExporterRates: [
      {
        exporterName: "Guangdong Annex Aluminium Producer",
        manufacturerName: "Guangdong Annex Aluminium Producer",
        additionalCode: "C998",
        ratePctBps: 750,
      },
    ],
  };
}

function adValoremMeasure(
  id: string,
  measureType: string,
  rateBps: number,
): ActiveMeasure {
  return {
    id,
    measureType,
    dutyExpression: { kind: "ad_valorem", rateBps },
  };
}

function exporterResolutionFixture() {
  return {
    commodityCode: "7308909800",
    countryOfOrigin: "CN",
    exporterRateBasis: "residual_default" as const,
    exporterIdentityConfidenceBps: 0,
    missingEvidence: [],
    evidenceLeverage: [],
  };
}
