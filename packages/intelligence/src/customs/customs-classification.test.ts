import { describe, expect, it } from "bun:test";

import { EXPORTER_UNRESOLVED, resolveExporter } from "./exporter-resolution";
import {
  InMemoryRecallLatticeStore,
  InMemoryTariffGraph,
  createRecallCandidate,
  expandRecallLatticeToTariffGraph,
  persistRecallLattice,
  recallLatticeRowsFromExpansion,
  verifyShortlistedTariffBranches,
  type MultiSourceRecallResult,
  type ProductCommodityCodeRow,
  type SourceFamily,
  type SourceFamilyResult,
  type TariffNode,
} from "./multi-source-recall";

describe("customs classification tariff graph", () => {
  it("L5-T-customs_classification-004 caches HMRC tariff graph data, verifies only shortlisted branches, and persists the recall lattice", async () => {
    const graph = new InMemoryTariffGraph(packagingTariffNodes());
    const recallResult = packagingRecallResult();

    const expansion = expandRecallLatticeToTariffGraph(graph, recallResult);

    expect(expansion.familySummaries.map((family) => family.familyCode)).toEqual([
      "3923",
      "4819",
      "4823",
    ]);
    expect(
      new Set(expansion.expandedCandidates.map((candidate) => candidate.familyCode)),
    ).toEqual(new Set(["3923", "4819", "4823"]));
    expect(expansion.selectedCandidate?.node.code).toBe("4819100000");
    expect(expansion.verificationQueue).toEqual([
      "3923100000",
      "4819100000",
      "4819200000",
      "4819300000",
      "4823908580",
    ]);

    const rejectedPlasticFamily = expansion.expandedCandidates.find(
      (candidate) => candidate.node.code === "3923100000",
    );
    expect(rejectedPlasticFamily?.rejectionReasons).toContain("material_mismatch");

    const selectedNode = graph.getNode("4819100000");
    expect(selectedNode).toMatchObject({
      supplementaryUnitRequirements: ["number_of_items"],
      additionalCodeRequirements: [
        {
          codeType: "VAT_ADDITIONAL_CODE",
          description: "UK VAT additional code for paperboard packaging",
          requiredWhen: "import_vat_measure_active",
        },
      ],
      measures: [
        {
          id: "mfn-4819",
          kind: "third_country_duty",
          dutyExpression: "0%",
        },
        {
          id: "vat-4819-standard",
          kind: "import_vat",
        },
      ],
    });

    const verifiedCodes: string[] = [];
    const verifications = await verifyShortlistedTariffBranches(expansion, (node) => {
      verifiedCodes.push(node.code);
      return {
        code: node.code,
        verifiedAt: "2026-05-19T22:50:00Z",
        measuresFreshUntil: "2026-05-26T00:00:00Z",
        measures: node.measures,
      };
    });

    expect(verifiedCodes).toEqual(expansion.verificationQueue);
    expect(verifiedCodes).not.toContain("4819400000");
    expect(verifications.every((verification) => verification.verifiedAt)).toBe(true);

    const store = new InMemoryRecallLatticeStore({
      rows: [oldProductCommodityCodeRow()],
      localSeq: 50,
    });
    const rows = await persistRecallLattice(store, {
      productCandidateId: "candidate-packaging-cartons",
      stageRunId: "stage-l5-new",
      recallResult,
      expandedCandidates: recallLatticeRowsFromExpansion(expansion),
      ownerId: "local-owner",
      commitGroupId: "l5-tariff-graph-expansion",
    });

    expect(store.rows).toHaveLength(rows.length);
    expect(store.rows.some((row) => row.stageRunId === "stage-l5-old")).toBe(false);
    expect(store.rows.every((row) => row.stageRunId === "stage-l5-new")).toBe(true);
    expect(store.rows.map((row) => row.commodityCode)).toContain("4819100000");
    expect(store.transactions).toEqual([
      {
        commitGroupId: "l5-tariff-graph-expansion",
        insertedRowIds: rows.map((row) => row.id),
      },
    ]);
    expect(new Set(store.outbox.map((entry) => entry.commitGroupId))).toEqual(
      new Set(["l5-tariff-graph-expansion"]),
    );
    expect(store.outbox).toHaveLength(store.rows.length);
    expect(store.outbox.every((entry) => entry.contentHash.startsWith("sha256:"))).toBe(
      true,
    );
    expect(
      store.rows.every((row) => evidenceIncludesCandidate(row.evidenceJson, "4819")),
    ).toBe(true);
  });
});

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

function evidenceIncludesCandidate(evidenceJson: string, candidateCode: string): boolean {
  try {
    const evidence = JSON.parse(evidenceJson) as {
      candidateRecallAgreement?: Array<{ candidateCode?: string }>;
    };

    return (
      evidence.candidateRecallAgreement?.some(
        (agreement) => agreement.candidateCode === candidateCode,
      ) ?? false
    );
  } catch (error) {
    throw new Error(`invalid evidence JSON: ${(error as Error).message}`);
  }
}

function packagingRecallResult(): MultiSourceRecallResult {
  const attributes = {
    headNoun: "cartons",
    material: "paperboard",
    principalFunction: "packaging",
    packForm: "bulk" as const,
  };
  const sourceFamilies: SourceFamily[] = [
    "hmrc_fpo_or_search_suggestion",
    "cpv_chapter_prior",
    "rule_based_chapter_hint",
    "hmrc_tariff_keyword_search",
    "atr_codebook_reuse",
  ];
  const byFamily = new Map<SourceFamily, SourceFamilyResult>([
    [
      "hmrc_fpo_or_search_suggestion",
      sourceResult("hmrc_fpo_or_search_suggestion", "agreed", "3923", 9_500),
    ],
    ["cpv_chapter_prior", sourceResult("cpv_chapter_prior", "agreed", "4819", 8_400)],
    [
      "rule_based_chapter_hint",
      sourceResult("rule_based_chapter_hint", "agreed", "4819", 8_800),
    ],
    [
      "hmrc_tariff_keyword_search",
      sourceResult("hmrc_tariff_keyword_search", "agreed", "4819", 9_200),
    ],
    ["atr_codebook_reuse", sourceResult("atr_codebook_reuse", "agreed", "4823", 7_100)],
  ]);

  return {
    attributes,
    perSourceResults: sourceFamilies.map((sourceFamily) => {
      const result = byFamily.get(sourceFamily);
      if (result === undefined) {
        throw new Error(`missing source result for ${sourceFamily}`);
      }
      return result;
    }),
    candidateRecallAgreement: [
      {
        candidateCode: "3923",
        agreement: sourceFamilies.map((sourceFamily) =>
          sourceFamily === "hmrc_fpo_or_search_suggestion"
            ? sourceResult(sourceFamily, "agreed", "3923", 9_500)
            : sourceResult(sourceFamily, "diverged", "4819", 8_000),
        ),
      },
      {
        candidateCode: "4819",
        agreement: sourceFamilies.map((sourceFamily) =>
          sourceFamily === "cpv_chapter_prior" ||
          sourceFamily === "rule_based_chapter_hint" ||
          sourceFamily === "hmrc_tariff_keyword_search"
            ? sourceResult(sourceFamily, "agreed", "4819", 9_200)
            : sourceResult(sourceFamily, "diverged", "3923", 8_000),
        ),
      },
      {
        candidateCode: "4823",
        agreement: sourceFamilies.map((sourceFamily) =>
          sourceFamily === "atr_codebook_reuse"
            ? sourceResult(sourceFamily, "agreed", "4823", 7_100)
            : sourceResult(sourceFamily, "diverged", "4819", 8_000),
        ),
      },
    ],
    independentSourceFamiliesAvailable: 5,
    independentSourceFamiliesAgreeing: 3,
    classificationBlockers: [],
    autoClassificationAllowed: true,
  };
}

function sourceResult(
  sourceFamily: SourceFamily,
  state: SourceFamilyResult["state"],
  code: string,
  scoreBps: number,
): SourceFamilyResult {
  return {
    sourceFamily,
    state,
    candidates: [
      createRecallCandidate({
        code,
        title: `${sourceFamily} candidate ${code}`,
        sourceFamily,
        retrievalScoreBps: scoreBps,
      }),
    ],
    scoreBps,
    failureMode: "none",
  };
}

function packagingTariffNodes(): TariffNode[] {
  const freshness = {
    structureCheckedAt: "2026-05-12T00:00:00Z",
    measuresFreshUntil: "2026-05-26T00:00:00Z",
  };

  return [
    tariffNode({
      code: "3923",
      parent: null,
      level: "heading",
      description: "Articles for the conveyance or packing of goods, of plastics",
      freshness,
    }),
    tariffNode({
      code: "3923100000",
      parent: "3923",
      level: "uk10",
      description: "Plastic boxes, cases, crates and similar articles",
      exclusions: ["Paperboard cartons and corrugated boxes are excluded"],
      measures: [
        {
          id: "mfn-3923",
          kind: "third_country_duty",
          description: "Third-country duty for plastic packing articles",
          dutyExpression: "6.5%",
        },
      ],
      freshness,
    }),
    tariffNode({
      code: "4819",
      parent: null,
      level: "heading",
      description:
        "Cartons, boxes, cases, bags and other packing containers, of paper or paperboard",
      freshness,
    }),
    tariffNode({
      code: "4819100000",
      parent: "4819",
      level: "uk10",
      description:
        "Cartons, boxes and cases of corrugated paper or paperboard for packaging",
      supplementaryUnitRequirements: ["number_of_items"],
      additionalCodeRequirements: [
        {
          codeType: "VAT_ADDITIONAL_CODE",
          description: "UK VAT additional code for paperboard packaging",
          requiredWhen: "import_vat_measure_active",
        },
      ],
      measures: [
        {
          id: "mfn-4819",
          kind: "third_country_duty",
          description: "Third-country duty for paperboard cartons",
          dutyExpression: "0%",
        },
        {
          id: "vat-4819-standard",
          kind: "import_vat",
          description: "Standard import VAT applies",
        },
      ],
      freshness,
    }),
    tariffNode({
      code: "4819200000",
      parent: "4819",
      level: "uk10",
      description:
        "Folding cartons, boxes and cases of non-corrugated paper or paperboard packaging",
      measures: [
        {
          id: "mfn-4819-folding",
          kind: "third_country_duty",
          description: "Third-country duty for folding paperboard cartons",
          dutyExpression: "0%",
        },
      ],
      freshness,
    }),
    tariffNode({
      code: "4819300000",
      parent: "4819",
      level: "uk10",
      description: "Sacks and bags of paper or paperboard for packing goods",
      freshness,
    }),
    tariffNode({
      code: "4819400000",
      parent: "4819",
      level: "uk10",
      description: "Other sacks and bags, including cone-shaped bags",
      freshness,
    }),
    tariffNode({
      code: "4823",
      parent: null,
      level: "heading",
      description: "Other paper, paperboard, cellulose wadding and webs",
      freshness,
    }),
    tariffNode({
      code: "4823908580",
      parent: "4823",
      level: "uk10",
      description: "Other paper and paperboard articles for packing support",
      measures: [
        {
          id: "mfn-4823",
          kind: "third_country_duty",
          description: "Third-country duty for other paper articles",
          dutyExpression: "0%",
        },
      ],
      freshness,
    }),
  ];
}

function tariffNode(node: Partial<TariffNode> & Pick<TariffNode, "code" | "description">): TariffNode {
  return {
    parent: null,
    level: "uk10",
    measures: [],
    supplementaryUnitRequirements: [],
    additionalCodeRequirements: [],
    scopeNotes: [],
    exclusions: [],
    freshness: {
      structureCheckedAt: "2026-05-12T00:00:00Z",
      measuresFreshUntil: "2026-05-26T00:00:00Z",
    },
    ...node,
  };
}

function oldProductCommodityCodeRow(): ProductCommodityCodeRow {
  return {
    id: "pcc_old",
    productCandidateId: "candidate-packaging-cartons",
    commodityCode: "3923100000",
    digits: 10,
    source: "hmrc_fpo_or_search_suggestion",
    scoreBps: 9_500,
    rankOrdinal: 1,
    description: "old plastic branch",
    evidenceJson: "{}",
    ownerId: "local-owner",
    rowVersion: 1,
    lastLocalSeq: 49,
    stageRunId: "stage-l5-old",
  };
}
