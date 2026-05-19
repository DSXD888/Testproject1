import { describe, expect, it } from "bun:test";

import {
  TRADE_REMEDY_WATCH_STALE,
  createTariffChangeEvent,
  createTradeRemedyCaseOpeningEvent,
  sweepFreshness,
} from "./classification-freshness";
import { EXPORTER_UNRESOLVED, resolveExporter } from "./exporter-resolution";
import { resolveVat } from "./measure-scanner";
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
import {
  TenDigitResolution,
  createTenDigitOperatorSurface,
} from "./ten-digit-resolution";

describe("customs classification tariff graph", () => {
  it("L5-T-customs_classification-001 rejects greedy descent and selects the compatible runner-up UK10 branch", () => {
    const result = new TenDigitResolution(
      new InMemoryTariffGraph(nonGreedyTariffNodes()),
    ).resolve(nonGreedyRecallResult().attributes, nonGreedyRecallResult());

    expect(result.greedyDescentDetected).toBe(false);
    expect(result.candidateCommodityCodesJson).toEqual([
      "3923100000",
      "4819100000",
      "4823908580",
    ]);
    expect(result.selectedUk10).toBe("4819100000");
    expect(result.rejectionReasonForTopRanked).toBe(
      "material mismatch on 3923 (rigid plastic vs paperboard)",
    );
    expect(result.latticeSeparation.recallFamilyCodes).toEqual([
      "3923",
      "4819",
      "4823",
    ]);
  });

  it("L5-T-customs_classification-002 reranks descendants with slot compatibility and source-family agreement states", () => {
    const recallResult = slotCompatibilityRecallResult();
    const result = new TenDigitResolution(
      new InMemoryTariffGraph(slotCompatibilityTariffNodes()),
    ).resolve(recallResult.attributes, recallResult);

    const selected = result.shortlisted.find(
      (candidate) => candidate.uk10Code === "4819100000",
    );

    expect(selected).toMatchObject({
      parentHs6: "4819",
      slotCompatibility: {
        descriptionMatchBps: 10_000,
        materialMatchBps: 10_000,
        unitCompatibility: "missing",
        exclusionsTriggered: [],
        scopeNoteFlags: [
          "Covers corrugated paperboard packing containers with declarable item counts",
        ],
      },
    });
    expect(selected?.candidateRecallAgreement.map((source) => source.state)).toEqual([
      "agreed",
      "agreed",
      "agreed",
      "diverged",
      "unavailable",
    ]);
    expect(result.perFamilyClusters[0]?.missingFacts).toEqual(
      expect.arrayContaining([
        "additional_code:VAT_ADDITIONAL_CODE",
        "supplementary_unit:number_of_items",
      ]),
    );
  });

  it("L5-T-customs_classification-003 keeps recall classification and commercial scenario lattices separated", () => {
    const recallResult = nonGreedyRecallResult();
    const result = new TenDigitResolution(
      new InMemoryTariffGraph(nonGreedyTariffNodes()),
    ).resolve(recallResult.attributes, recallResult);

    expect(result.latticeSeparation).toMatchObject({
      recallFamilyCodes: ["3923", "4819", "4823"],
      classificationCandidateCodes: [
        "3923100000",
        "4819100000",
        "4823908580",
      ],
      shortlistedBeforeCommercialScenarios: true,
      commercialScenarioInputsUsed: [],
    });
    expect(
      result.shortlisted.every(
        (candidate) =>
          !Object.hasOwn(candidate, "freight") &&
          !Object.hasOwn(candidate, "fx") &&
          !Object.hasOwn(candidate, "quoteRole"),
      ),
    ).toBe(true);
  });

  it("L5-T-customs_classification-005 groups large descendant sets into operator family clusters", () => {
    const recallResult = largePaperboardRecallResult();
    const result = new TenDigitResolution(
      new InMemoryTariffGraph(largePaperboardTariffNodes()),
    ).resolve(recallResult.attributes, recallResult);
    const surface = createTenDigitOperatorSurface(result);

    expect(surface.kind).toBe("family_clusters");
    expect(surface.clusters).toHaveLength(1);
    expect(surface.clusters[0]).toMatchObject({
      familyLabel:
        "4819 - Cartons, boxes, cases, bags and other packing containers, of paper or paperboard",
      count: 55,
      conservativeDutyOutcome: {
        dutyPctBps: 1_250,
        rationale: "highest rate-bearing measure in expanded HMRC family",
      },
      mostSupportedBranch: {
        uk10Code: "4819100000",
        parentHs6: "4819",
      },
      missingFacts: ["supplementary_unit:number_of_items"],
    });
    expect(() => createTenDigitOperatorSurface(result, { mode: "flat_list" })).toThrow(
      "flat ungrouped UK10 lists are not an acceptable operator surface",
    );
  });

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

describe("customs classification freshness", () => {
  it("L5-T-customs_classification-022 tracks structure, measures, remedies, and FX independently", () => {
    const result = sweepFreshness({
      uk10: "4819100000",
      vectors: freshnessVectors({
        tariffCheckedAt: "2026-05-18T00:00:00Z",
        measuresFreshUntil: "2026-05-21T00:00:00Z",
        tradeRemedyCheckedAt: "2026-05-18T00:00:00Z",
        fxRateCheckedAt: "2026-05-19T00:00:00Z",
      }),
      expectedImportWindow: {
        start: "2026-05-22T00:00:00Z",
        end: "2026-05-29T00:00:00Z",
      },
      now: new Date("2026-05-19T12:00:00Z"),
      configuredWindow: {
        structureDays: 14,
        measuresDays: 7,
        remedyDays: 14,
        fxDays: 31,
      },
    });

    expect(result.passing).toBe(false);
    expect(result.expiredVectors).toEqual(["measures"]);
    expect(result.vectorStates).toMatchObject({
      structure: { passing: true },
      measures: { passing: false },
      remedies: { passing: true },
      fx: { passing: true },
    });
    expect(result.enqueueReclassification).toBe(true);
    expect(result.enqueueRecalculation).toBe(false);
    expect(result.blocksL6Readiness).toBe(true);

    const remedyResult = sweepFreshness({
      uk10: "7604210000",
      vectors: freshnessVectors({
        tariffCheckedAt: "2026-05-18T00:00:00Z",
        measuresFreshUntil: "2026-06-15T00:00:00Z",
        tradeRemedyWatchState: "investigation_open",
        tradeRemedyCheckedAt: "2026-05-18T00:00:00Z",
        fxRateCheckedAt: "2026-05-19T00:00:00Z",
      }),
      expectedImportWindow: {
        start: "2026-05-22T00:00:00Z",
        end: "2026-05-29T00:00:00Z",
      },
      now: new Date("2026-05-19T12:00:00Z"),
      configuredWindow: {
        structureDays: 14,
        measuresDays: 30,
        remedyDays: 14,
        fxDays: 31,
      },
    });

    expect(remedyResult.expiredVectors).toEqual(["remedies"]);
    expect(remedyResult.blockers).toContain(TRADE_REMEDY_WATCH_STALE);
    expect(remedyResult.vectorStates.fx.passing).toBe(true);
    expect(remedyResult.enqueueReclassification).toBe(true);
    expect(remedyResult.enqueueRecalculation).toBe(false);
  });

  it("L5-T-customs_classification-023 emits tariff and TRA watcher events for affected UK10 codes", () => {
    const tariffEvent = createTariffChangeEvent({
      kind: "structure",
      changedAt: "2026-05-19T03:15:00Z",
      affectedUk10Codes: ["4819100000", "4819200000"],
      sourceReference: "hmrc-tariff-news-2026-05-19",
    });

    expect(tariffEvent).toMatchObject({
      eventCode: "TARIFF_STRUCTURE_CHANGED",
      vector: "structure",
      enqueueReclassification: true,
      enqueueRecalculation: false,
      affectedUk10Codes: ["4819100000", "4819200000"],
      affectedCommodityFamilies: ["4819"],
    });

    const remedyEvent = createTradeRemedyCaseOpeningEvent({
      openedAt: "2026-05-19T02:45:00Z",
      caseReference: "TRA/2026/ACME-ALUMINIUM",
      affectedUk10Codes: ["7604210000"],
      originCountries: ["CN"],
    });

    expect(remedyEvent).toMatchObject({
      eventCode: "TRA_INVESTIGATION_OPEN",
      vector: "remedies",
      tradeRemedyWatchState: "investigation_open",
      enqueueReclassification: true,
      enqueueRecalculation: false,
      blockers: [TRADE_REMEDY_WATCH_STALE],
      affectedCommodityFamilies: ["7604"],
    });
  });

  it("L5-T-customs_classification-024 marks missed watchers stale and blocks readiness until catch-up completes", () => {
    const stale = sweepFreshness({
      uk10: "4819100000",
      vectors: freshnessVectors({
        tariffCheckedAt: "2026-05-18T00:00:00Z",
        measuresFreshUntil: "2026-06-15T00:00:00Z",
        tradeRemedyCheckedAt: "2026-05-18T00:00:00Z",
        fxRateCheckedAt: "2026-05-18T00:00:00Z",
      }),
      expectedImportWindow: {
        start: "2026-05-20T00:00:00Z",
        end: "2026-05-24T00:00:00Z",
      },
      now: new Date("2026-05-19T12:00:00Z"),
      configuredWindow: {
        structureDays: 14,
        measuresDays: 14,
        remedyDays: 1,
        fxDays: 31,
      },
      lastSuccessfulWatcherRuns: {
        remedies: "2026-05-17T02:45:00Z",
      },
    });

    expect(stale.expiredVectors).toEqual(["remedies"]);
    expect(stale.tradeRemedyWatchState).toBe("stale");
    expect(stale.catchUpJobs).toEqual([
      {
        action: "reclassify",
        affectedCommodityFamilies: ["4819"],
        affectedUk10Codes: ["4819100000"],
        reason: "watcher_missed_window",
        vector: "remedies",
      },
    ]);
    expect(stale.blockers).toContain(TRADE_REMEDY_WATCH_STALE);
    expect(stale.blocksL6Readiness).toBe(true);

    const caughtUp = sweepFreshness({
      uk10: "4819100000",
      vectors: freshnessVectors({
        tariffCheckedAt: "2026-05-19T11:00:00Z",
        measuresFreshUntil: "2026-06-15T00:00:00Z",
        tradeRemedyCheckedAt: "2026-05-19T11:00:00Z",
        fxRateCheckedAt: "2026-05-19T11:00:00Z",
      }),
      expectedImportWindow: {
        start: "2026-05-20T00:00:00Z",
        end: "2026-05-24T00:00:00Z",
      },
      now: new Date("2026-05-19T12:00:00Z"),
      configuredWindow: {
        structureDays: 14,
        measuresDays: 14,
        remedyDays: 14,
        fxDays: 31,
      },
      lastSuccessfulWatcherRuns: {
        remedies: "2026-05-19T11:00:00Z",
      },
    });

    expect(caughtUp.passing).toBe(true);
    expect(caughtUp.catchUpJobs).toEqual([]);
    expect(caughtUp.blocksL6Readiness).toBe(false);
  });
});

describe("marketplace supplier quote coexistence", () => {
  it("L5-T-customs_classification-008 keeps Alibaba 1688 Amazon and eBay rows with source-specific evidence roles", () => {
    const quotes = marketplaceSupplierQuotes();
    const roleBySource: Record<MarketplaceSourceId, MarketplacePriceSignalRole> = {
      ebay: "benchmark_ceiling",
      alibaba: "optimistic_floor",
      "1688": "domestic_wholesale_floor",
      amazon: "benchmark_ceiling",
    };

    expect(new Set(quotes.map((quote) => quote.sourceId))).toEqual(
      new Set(["ebay", "alibaba", "1688", "amazon"]),
    );

    for (const quote of quotes) {
      expect(quote.priceSignalRole, quote.sourceId).toBe(
        roleBySource[quote.sourceId],
      );
      expect(quote.executableWholesaleQuote, quote.sourceId).toBe(false);
      expect(quote.countryOfOrigin, quote.sourceId).toBeTruthy();
      expect(quote.evidence.listingUrl, quote.sourceId).toMatch(/^https:\/\//);
      expect(quote.evidence.transcriptPath, quote.sourceId).toContain(
        quote.sourceId,
      );
      expect(quote.evidence.originEvidenceState, quote.sourceId).toBe(
        "source_observed",
      );
      expect(quote.quoteUsabilityConfidenceBps, quote.sourceId).toBeGreaterThan(
        0,
      );
      expect(quote.matchConfidenceBps, quote.sourceId).toBeGreaterThan(0);
    }
  });

  it("L5-T-customs_classification-011 requires every validated category to have eBay plus at least one non-eBay marketplace source", () => {
    const quotes = marketplaceSupplierQuotes();
    const categories: MarketplaceCategory[] = [
      "fasteners",
      "packaging-cartons",
      "examination-gloves",
    ];

    for (const category of categories) {
      const sources = quotes
        .filter((quote) => quote.category === category)
        .map((quote) => quote.sourceId);
      expect(sources, category).toContain("ebay");
      expect(
        sources.some((source) => ["alibaba", "1688", "amazon"].includes(source)),
        category,
      ).toBe(true);
    }
  });
});

describe("customs classification exporter defaults", () => {
  it("L5-T-customs_classification-016 records unresolved VAT as a standard-rate conservative default", () => {
    const result = resolveVat({
      uk10: "4819100000",
      measures: [],
      reliefClaims: [{ kind: "family_6_17", evidenceArtifactPaths: [] }],
    });

    expect(result).toMatchObject({
      vatRateState: "unresolved",
      importVatTreatment: "standard_rate_default",
      vatRecoveryRateBps: 0,
    });
    expect(result.evidenceLeverage).toEqual(
      expect.arrayContaining([
        {
          leverageItem: "vat_relief_family_6_17",
          marginDeltaBps: 2_000,
          evidenceRequired: [
            "commodity_prerequisite",
            "documentary_relief_evidence",
          ],
        },
        {
          leverageItem: "vat_rate_resolution",
          marginDeltaBps: 2_000,
          evidenceRequired: ["hmrc_import_vat_measure"],
        },
      ]),
    );
  });

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

type MarketplaceCategory =
  | "fasteners"
  | "packaging-cartons"
  | "examination-gloves";

type MarketplaceSourceId = "ebay" | "alibaba" | "1688" | "amazon";

type MarketplacePriceSignalRole =
  | "benchmark_ceiling"
  | "optimistic_floor"
  | "domestic_wholesale_floor";

type MarketplaceSupplierQuote = {
  category: MarketplaceCategory;
  sourceId: MarketplaceSourceId;
  sourceName: string;
  supplierName: string;
  priceSignalRole: MarketplacePriceSignalRole;
  executableWholesaleQuote: boolean;
  countryOfOrigin: string;
  matchConfidenceBps: number;
  quoteUsabilityConfidenceBps: number;
  evidence: {
    listingUrl: string;
    transcriptPath: string;
    originEvidenceState: "source_observed";
  };
};

function marketplaceSupplierQuotes(): MarketplaceSupplierQuote[] {
  return [
    marketplaceQuote("fasteners", "ebay", "eBay", "Bolt Warehouse UK", {
      countryOfOrigin: "GB",
      priceSignalRole: "benchmark_ceiling",
    }),
    marketplaceQuote("fasteners", "alibaba", "Alibaba", "Ningbo Fastener Factory", {
      countryOfOrigin: "CN",
      priceSignalRole: "optimistic_floor",
    }),
    marketplaceQuote("packaging-cartons", "ebay", "eBay", "Packaging Supplies UK", {
      countryOfOrigin: "GB",
      priceSignalRole: "benchmark_ceiling",
    }),
    marketplaceQuote(
      "packaging-cartons",
      "1688",
      "1688",
      "义乌包装制品有限公司",
      {
        countryOfOrigin: "CN",
        priceSignalRole: "domestic_wholesale_floor",
      },
    ),
    marketplaceQuote(
      "packaging-cartons",
      "amazon",
      "Amazon",
      "Amazon Business Seller",
      {
        countryOfOrigin: "GB",
        priceSignalRole: "benchmark_ceiling",
      },
    ),
    marketplaceQuote(
      "examination-gloves",
      "ebay",
      "eBay",
      "Medical Consumables UK",
      {
        countryOfOrigin: "GB",
        priceSignalRole: "benchmark_ceiling",
      },
    ),
    marketplaceQuote(
      "examination-gloves",
      "amazon",
      "Amazon",
      "Amazon Healthcare Seller",
      {
        countryOfOrigin: "GB",
        priceSignalRole: "benchmark_ceiling",
      },
    ),
  ];
}

function marketplaceQuote(
  category: MarketplaceCategory,
  sourceId: MarketplaceSourceId,
  sourceName: string,
  supplierName: string,
  options: Pick<MarketplaceSupplierQuote, "countryOfOrigin" | "priceSignalRole">,
): MarketplaceSupplierQuote {
  return {
    category,
    sourceId,
    sourceName,
    supplierName,
    priceSignalRole: options.priceSignalRole,
    executableWholesaleQuote: false,
    countryOfOrigin: options.countryOfOrigin,
    matchConfidenceBps: 8_900,
    quoteUsabilityConfidenceBps: 8_600,
    evidence: {
      listingUrl: `https://example.test/${sourceId}/${category}`,
      transcriptPath: `tests/fixtures/cassettes/l5/marketplace/${sourceId}/${category}.json`,
      originEvidenceState: "source_observed",
    },
  };
}

function freshnessVectors(
  overrides: Partial<Parameters<typeof sweepFreshness>[0]["vectors"]>,
): Parameters<typeof sweepFreshness>[0]["vectors"] {
  return {
    tariffCheckedAt: "2026-05-18T00:00:00Z",
    measuresFreshUntil: "2026-06-15T00:00:00Z",
    tradeRemedyWatchState: "stable",
    tradeRemedyCheckedAt: "2026-05-18T00:00:00Z",
    fxRateCheckedAt: "2026-05-19T00:00:00Z",
    ...overrides,
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

function nonGreedyRecallResult(): MultiSourceRecallResult {
  const sourceFamilies: SourceFamily[] = [
    "hmrc_fpo_or_search_suggestion",
    "cpv_chapter_prior",
    "rule_based_chapter_hint",
    "hmrc_tariff_keyword_search",
    "atr_codebook_reuse",
  ];

  return {
    attributes: packagingAttributes(),
    perSourceResults: [
      sourceResult("hmrc_fpo_or_search_suggestion", "agreed", "3923", 9_700),
      sourceResult("cpv_chapter_prior", "agreed", "4819", 8_400),
      sourceResult("rule_based_chapter_hint", "agreed", "4819", 8_800),
      sourceResult("hmrc_tariff_keyword_search", "agreed", "4819", 9_200),
      sourceResult("atr_codebook_reuse", "agreed", "4823", 7_100),
    ],
    candidateRecallAgreement: [
      {
        candidateCode: "3923",
        agreement: sourceFamilies.map((sourceFamily) =>
          sourceFamily === "hmrc_fpo_or_search_suggestion"
            ? sourceResult(sourceFamily, "agreed", "3923", 9_700)
            : sourceResult(sourceFamily, "diverged", "4819", 7_000),
        ),
      },
      {
        candidateCode: "4819",
        agreement: sourceFamilies.map((sourceFamily) =>
          sourceFamily === "cpv_chapter_prior" ||
          sourceFamily === "rule_based_chapter_hint" ||
          sourceFamily === "hmrc_tariff_keyword_search"
            ? sourceResult(sourceFamily, "agreed", "4819", 9_200)
            : sourceResult(sourceFamily, "diverged", "3923", 7_000),
        ),
      },
      {
        candidateCode: "4823",
        agreement: sourceFamilies.map((sourceFamily) =>
          sourceFamily === "atr_codebook_reuse"
            ? sourceResult(sourceFamily, "agreed", "4823", 7_100)
            : sourceResult(sourceFamily, "diverged", "4819", 7_000),
        ),
      },
    ],
    independentSourceFamiliesAvailable: 5,
    independentSourceFamiliesAgreeing: 3,
    classificationBlockers: [],
    autoClassificationAllowed: true,
  };
}

function slotCompatibilityRecallResult(): MultiSourceRecallResult {
  return {
    ...nonGreedyRecallResult(),
    candidateRecallAgreement: [
      {
        candidateCode: "4819",
        agreement: [
          sourceResult("hmrc_fpo_or_search_suggestion", "agreed", "4819", 9_000),
          sourceResult("cpv_chapter_prior", "agreed", "4819", 8_700),
          sourceResult("rule_based_chapter_hint", "agreed", "4819", 8_900),
          sourceResult("hmrc_tariff_keyword_search", "diverged", "3923", 8_000),
          unavailableSourceResult("atr_codebook_reuse"),
        ],
      },
    ],
    perSourceResults: [
      sourceResult("hmrc_fpo_or_search_suggestion", "agreed", "4819", 9_000),
      sourceResult("cpv_chapter_prior", "agreed", "4819", 8_700),
      sourceResult("rule_based_chapter_hint", "agreed", "4819", 8_900),
      sourceResult("hmrc_tariff_keyword_search", "diverged", "3923", 8_000),
      unavailableSourceResult("atr_codebook_reuse"),
    ],
  };
}

function largePaperboardRecallResult(): MultiSourceRecallResult {
  return {
    ...slotCompatibilityRecallResult(),
    candidateRecallAgreement: [
      {
        candidateCode: "4819",
        agreement: [
          sourceResult("hmrc_fpo_or_search_suggestion", "agreed", "4819", 9_000),
          sourceResult("cpv_chapter_prior", "agreed", "4819", 8_700),
          sourceResult("rule_based_chapter_hint", "agreed", "4819", 8_900),
          sourceResult("hmrc_tariff_keyword_search", "agreed", "4819", 9_200),
          sourceResult("atr_codebook_reuse", "agreed", "4819", 7_500),
        ],
      },
    ],
  };
}

function packagingAttributes() {
  return {
    headNoun: "cartons",
    material: "paperboard",
    principalFunction: "packaging",
    packForm: "bulk" as const,
  };
}

function packagingRecallResult(): MultiSourceRecallResult {
  const attributes = packagingAttributes();
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

function unavailableSourceResult(sourceFamily: SourceFamily): SourceFamilyResult {
  return {
    sourceFamily,
    state: "unavailable",
    candidates: [],
    scoreBps: null,
    failureMode: "downtime",
  };
}

function nonGreedyTariffNodes(): TariffNode[] {
  const freshness = {
    structureCheckedAt: "2026-05-12T00:00:00Z",
    measuresFreshUntil: "2026-05-26T00:00:00Z",
  };

  return [
    tariffNode({
      code: "3923",
      parent: null,
      level: "heading",
      description: "Articles for packing goods, of rigid plastic",
      freshness,
    }),
    tariffNode({
      code: "3923100000",
      parent: "3923",
      level: "uk10",
      description: "Rigid plastic boxes, cases and crates for packing goods",
      exclusions: ["Paperboard cartons and corrugated boxes are excluded"],
      measures: [
        {
          id: "mfn-3923",
          kind: "third_country_duty",
          description: "Third-country duty for rigid plastic packaging",
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
      description: "Corrugated paperboard cartons and boxes for packaging goods",
      measures: [
        {
          id: "mfn-4819",
          kind: "third_country_duty",
          description: "Third-country duty for corrugated paperboard packaging",
          dutyExpression: "0%",
        },
      ],
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
      description: "Other paperboard articles for packing support",
      measures: [
        {
          id: "mfn-4823",
          kind: "third_country_duty",
          description: "Third-country duty for other paperboard articles",
          dutyExpression: "0%",
        },
      ],
      freshness,
    }),
  ];
}

function slotCompatibilityTariffNodes(): TariffNode[] {
  return [
    tariffNode({
      code: "4819",
      parent: null,
      level: "heading",
      description:
        "Cartons, boxes, cases, bags and other packing containers, of paper or paperboard",
    }),
    tariffNode({
      code: "4819100000",
      parent: "4819",
      level: "uk10",
      description:
        "Cartons and boxes of corrugated paper or paperboard for packaging goods",
      scopeNotes: [
        "Covers corrugated paperboard packing containers with declarable item counts",
      ],
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
          description: "Third-country duty for corrugated paperboard packaging",
          dutyExpression: "0%",
        },
        {
          id: "vat-4819-standard",
          kind: "import_vat",
          description: "Standard import VAT applies to paperboard packaging",
        },
      ],
    }),
    tariffNode({
      code: "4819200000",
      parent: "4819",
      level: "uk10",
      description: "Folding boxes of non-corrugated paperboard",
      exclusions: ["Corrugated paperboard cartons are excluded"],
      measures: [
        {
          id: "mfn-4819-folding",
          kind: "third_country_duty",
          description: "Third-country duty for folding paperboard boxes",
          dutyExpression: "0%",
        },
      ],
    }),
  ];
}

function largePaperboardTariffNodes(): TariffNode[] {
  const heading = tariffNode({
    code: "4819",
    parent: null,
    level: "heading",
    description:
      "Cartons, boxes, cases, bags and other packing containers, of paper or paperboard",
  });
  const descendants = Array.from({ length: 55 }, (_, index) => {
    const code = index === 0 ? "4819100000" : `481910${String(index).padStart(4, "0")}`;
    return tariffNode({
      code,
      parent: "4819",
      level: "uk10",
      description:
        index === 0
          ? "Corrugated paperboard cartons and boxes for packaging goods"
          : `Paperboard packaging carton variant ${index}`,
      supplementaryUnitRequirements: ["number_of_items"],
      measures: [
        {
          id: `mfn-4819-${index}`,
          kind: "third_country_duty",
          description: "Third-country duty for paperboard packaging",
          dutyExpression: index === 54 ? "12.5%" : "0%",
        },
      ],
    });
  });

  return [heading, ...descendants];
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
