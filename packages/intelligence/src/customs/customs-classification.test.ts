import { describe, expect, it } from "bun:test";

import { EXPORTER_UNRESOLVED, resolveExporter } from "./exporter-resolution";

describe("customs classification exporter defaults", () => {
  it("L5-T-customs_classification-014 applies residual default and unlockable margin for missing exporter proof", () => {
    const result = resolveExporter({
      commodityCode: "7604210000",
      countryOfOrigin: "CN",
      customsValueGbpPence: 1_000_000,
      supplierQuotes: [
        {
          supplierName: "Guangzhou Metals Distributor Ltd",
          supplierRole: "distributor",
        },
      ],
      measuresAtCommodity: [aluminiumExtrusionMeasure()],
    });

    expect(result).toMatchObject({
      exporterRateBasis: "residual_default",
      exporterIdentityConfidenceBps: 1500,
      selectedRatePctBps: 3510,
      appliedTradeRemedyDutyGbpPence: 351_000,
      unlockableMarginBps: 3510,
      errorCode: EXPORTER_UNRESOLVED,
    });
    expect(result.exporterIdentityConfidenceBps).toBeLessThanOrEqual(2500);
    expect(result.missingEvidence).toEqual(
      expect.arrayContaining([
        "distributor_is_not_exporter_proof",
        "eligible_manufacturer_or_exporter_name",
        "invoice_or_certificate",
        "supplier_profile",
      ]),
    );
    expect(result.evidenceLeverage).toEqual([
      {
        kind: "named_exporter_proof",
        missingEvidence: result.missingEvidence,
        unlockableMarginBps: 3510,
      },
    ]);
  });
});

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
