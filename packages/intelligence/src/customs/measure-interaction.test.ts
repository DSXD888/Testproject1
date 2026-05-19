import { describe, expect, it } from "bun:test";

import { EXPORTER_UNRESOLVED, resolveExporter } from "./exporter-resolution";

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
