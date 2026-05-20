import { describe, expect, it } from "bun:test";

import {
  CUSTOMS_ACCURACY_MUTATION_FAMILIES,
  CUSTOMS_SPECIFIC_MUTATION_DIMENSIONS,
  auditMutationTaxonomy,
  calculateDemotionThresholdBps,
  calculateRuleOfThreeUpperBoundBps,
  calculateWilsonLowerBoundBps,
  scoreCategory,
  toAccuracyHarnessMetricContract,
  type AccuracyCase,
} from "./accuracy";
import {
  TRADE_REMEDY_WATCH_STALE,
  createTariffChangeEvent,
  createTradeRemedyCaseOpeningEvent,
  evaluateCbamExposure,
  sweepFreshness,
} from "./classification-freshness";
import { EXPORTER_UNRESOLVED, resolveExporter } from "./exporter-resolution";
import {
  CLASSIFICATION_FRESH_UNTIL_EXPIRED,
  EXPORTER_UNRESOLVED_BLOCKER,
  FLOOR_ONLY_QUOTE_ROLE,
  SUPPLEMENTARY_UNIT_MISSING,
  computeConfidenceBreakdown,
  classifyDecisionGrade,
  resolveVat,
  scanMeasures,
  type DecisionGradeInput,
} from "./measure-scanner";
import type { FreightEngineOutputs } from "../logistics/freight-engine";
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
  CLASSIFICATION_AMBIGUOUS,
  TARIFF_BRANCH_DIVERGENCE,
  TenDigitResolution,
  createTenDigitOperatorSurface,
  detectBranchDivergence,
  type TenDigitCandidate,
} from "./ten-digit-resolution";

const repoRoot = new URL("../../../../", import.meta.url);
const sqliteMigrationUrl = new URL(
  "migrations/0001_sqlite_state_initial.sql",
  repoRoot,
);
const productCommodityCodesMigrationUrl = new URL(
  "migrations/0013_l5_product_commodity_codes.sql",
  repoRoot,
);
const packagingCartonsCorpusRootUrl = new URL(
  "tests/fixtures/calibration/customs-classification/packaging-cartons/",
  repoRoot,
);

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

  it("L5-T-customs_classification-006 set-replaces all product commodity code rows under one stage_run_id", async () => {
    const expansion = expandRecallLatticeToTariffGraph(
      new InMemoryTariffGraph(packagingTariffNodes()),
      packagingRecallResult(),
    );
    const store = new InMemoryRecallLatticeStore({
      rows: [oldProductCommodityCodeRow()],
      localSeq: 200,
    });

    const rows = await persistRecallLattice(store, {
      productCandidateId: "candidate-packaging-cartons",
      stageRunId: "stage-l5-replacement",
      recallResult: packagingRecallResult(),
      expandedCandidates: recallLatticeRowsFromExpansion(expansion),
      ownerId: "local-owner",
      commitGroupId: "l5-product-code-replacement",
    });

    expect(store.rows).toHaveLength(rows.length);
    expect(store.rows.some((row) => row.stageRunId === "stage-l5-old")).toBe(false);
    expect(store.rows.every((row) => row.stageRunId === "stage-l5-replacement")).toBe(
      true,
    );
    expect(new Set(store.transactions.map((tx) => tx.commitGroupId))).toEqual(
      new Set(["l5-product-code-replacement"]),
    );
    expect(store.outbox).toHaveLength(rows.length);
    expect(
      store.outbox.every(
        (entry) =>
          entry.entityKind === "product_commodity_codes" &&
          entry.commitGroupId === "l5-product-code-replacement" &&
          entry.contentHash.startsWith("sha256:"),
      ),
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

describe("customs classification CBAM placeholder", () => {
  it("L5-T-customs_classification-025 flags confirmed 2027 sectors without quantifying CBAM", () => {
    const inScopeCases = [
      { sector: "iron_and_steel", uk10: "7318159590" },
      { sector: "aluminium", uk10: "7607111900" },
      { sector: "cement", uk10: "2523290000" },
      { sector: "fertiliser", uk10: "3102101000" },
      { sector: "hydrogen", uk10: "2804100000" },
    ] as const;

    for (const fixture of inScopeCases) {
      const result = evaluateCbamExposure({
        uk10: fixture.uk10,
        deliveryWindow: {
          start: "2026-12-15T00:00:00Z",
          end: "2027-01-15T00:00:00Z",
        },
      });

      expect(result).toMatchObject({
        cbamExposureFlag: 1,
        cbamEstimateGbpPence: null,
        sector: fixture.sector,
        rationale:
          "confirmed_2027_sector_window_crosses_2027_01_01_estimate_pending_secondary_legislation",
      });
    }
  });

  it("L5-T-customs_classification-026 clears non-scope families and pre-2027 windows", () => {
    const clearCases = [
      {
        uk10: "7010904300",
        deliveryWindow: {
          start: "2026-12-15T00:00:00Z",
          end: "2027-01-15T00:00:00Z",
        },
        sector: null,
        rationale: "outside_confirmed_2027_cbam_sector_scope",
      },
      {
        uk10: "6911100000",
        deliveryWindow: {
          start: "2026-12-15T00:00:00Z",
          end: "2027-01-15T00:00:00Z",
        },
        sector: null,
        rationale: "outside_confirmed_2027_cbam_sector_scope",
      },
      {
        uk10: "7318159590",
        deliveryWindow: {
          start: "2026-12-01T00:00:00Z",
          end: "2026-12-31T23:59:59Z",
        },
        sector: "iron_and_steel",
        rationale: "delivery_window_ends_before_2027_01_01",
      },
      {
        uk10: "7607111900",
        deliveryWindow: {
          start: "2026-12-01T00:00:00Z",
          end: "2026-12-31T23:59:59Z",
        },
        sector: "aluminium",
        rationale: "delivery_window_ends_before_2027_01_01",
      },
    ] as const;

    for (const fixture of clearCases) {
      const result = evaluateCbamExposure({
        uk10: fixture.uk10,
        deliveryWindow: fixture.deliveryWindow,
      });

      expect(result).toMatchObject({
        cbamExposureFlag: 0,
        cbamEstimateGbpPence: null,
        sector: fixture.sector,
        rationale: fixture.rationale,
      });
    }
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

describe("customs classification phase gate fixtures", () => {
  it("L5-T-customs_classification-007 activates decision-grade landed-cost fields without a schema fork", async () => {
    const [sqliteMigration, productCommodityCodesMigration] = await Promise.all([
      readText(sqliteMigrationUrl),
      readText(productCommodityCodesMigrationUrl),
    ]);
    const columns = await landedCostEstimateColumns();

    for (const column of L5_LANDED_COST_COLUMN_CONTRACT) {
      expect(columns).toContain(column);
    }
    expect(new Set(columns).size).toBe(columns.length);
    expect(productCommodityCodesMigration).toContain(
      "CREATE TABLE IF NOT EXISTS product_commodity_codes",
    );
    expect(productCommodityCodesMigration).not.toContain(
      "ALTER TABLE landed_cost_estimates",
    );
    expect(sqliteMigration).toContain("CREATE TABLE IF NOT EXISTS landed_cost_estimates");
  });

  it("L5-T-customs_classification-009 produces decision-grade evidence bundles for all validated categories", () => {
    const bundles = decisionGradeCategoryBundles();

    expect(bundles.map((bundle) => bundle.category)).toEqual([
      "fasteners",
      "packaging-cartons",
      "examination-gloves",
    ]);
    for (const bundle of bundles) {
      expect(bundle.customs.declarableCommodityCode10).toMatch(/^\d{10}$/);
      expect(bundle.customs.dutyBreakdownJson.components.length, bundle.category).toBeGreaterThan(0);
      expect(bundle.fx.customsFxRateCheckedAt, bundle.category).toBeTruthy();
      expect(bundle.freight.providerEvidence, bundle.category).toMatch(/^(freightos|searates)$/);
      expect(bundle.vat.importVatTreatment, bundle.category).toBeTruthy();
      expect(bundle.confidenceBreakdown.evidence_leverage.length, bundle.category).toBeGreaterThan(0);
      expect(bundle.blockers, bundle.category).toEqual(expect.any(Array));
      expect(bundle.decisionGrade, bundle.category).toMatch(
        /^(auto_pass|review_required|broker_required)$/,
      );
    }
  });

  it("L5-T-customs_classification-010 blocks category promotion without evidence-tiered customs corpus coverage", () => {
    const gates = [
      categoryGate("fasteners", { corpusExists: true, goldLabels: 12, silverLabels: 18 }),
      categoryGate("packaging-cartons", {
        corpusExists: true,
        goldLabels: 10,
        silverLabels: 20,
      }),
      categoryGate("examination-gloves", {
        corpusExists: false,
        goldLabels: 0,
        silverLabels: 0,
      }),
    ];

    expect(gates[0]).toMatchObject({ productionReady: true, blockers: [] });
    expect(gates[1]).toMatchObject({ productionReady: true, blockers: [] });
    expect(gates[2]).toMatchObject({
      productionReady: false,
    });
    expect(gates[2]?.blockers).toEqual([
      "customs_classification_corpus_missing",
      "evidence_tier_metrics_insufficient",
    ]);
  });

  it("L5-T-customs_classification-012 runs recall, UK10, exporter, measures, VAT, freshness, and defaults before net-margin gating", () => {
    const recallResult = packagingRecallResult();
    const resolution = new TenDigitResolution(
      new InMemoryTariffGraph(packagingTariffNodes()),
    ).resolve(recallResult.attributes, recallResult);
    const selectedUk10 = resolution.selectedUk10 ?? "4819100000";
    const exporterResolution = noTradeRemedyExporterResolution(selectedUk10);
    const measureScan = scanMeasures({
      uk10: selectedUk10,
      measures: [
        {
          id: "mfn-paperboard",
          measureType: "MFN",
          dutyExpression: { kind: "ad_valorem", rateBps: 0 },
        },
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
      supplementaryQuantities: { number_of_items: 1_000 },
      exporterResolution,
    });
    const vatResolution = resolveVat({
      uk10: selectedUk10,
      measures: [],
      reliefClaims: [{ kind: "family_6_17", evidenceArtifactPaths: [] }],
    });
    const freshnessSweep = decisionFreshnessSweep();
    const confidenceBreakdown = computeConfidenceBreakdown({
      originPreference: { originProofAvailable: false },
      exporterResolution,
      vatResolution,
      measureScanResult: measureScan,
      marginSnapshot: {
        snapshotId: "full-pipeline-before-net-margin",
        netMarginBaseBps: 1_600,
        resolvedNetMarginBaseBps: {
          origin_preference: 1_900,
          vat_rate: 1_800,
        },
      },
    });
    const decision = classifyDecisionGrade({
      ...baseDecisionGradeInput(),
      measureScan,
      freshnessSweep,
      worstCaseNetMarginBps: 1_600,
    });

    expect(recallResult.independentSourceFamiliesAvailable).toBe(5);
    expect(resolution.selectedUk10).toBe("4819100000");
    expect(exporterResolution.exporterRateBasis).toBe("residual_default");
    expect(measureScan.additionalCodesRequirementsJson[0]?.satisfiability).toBe(
      "unknown",
    );
    expect(vatResolution.importVatTreatment).toBe("standard_rate_default");
    expect(freshnessSweep.passing).toBe(true);
    expect(confidenceBreakdown.active_defaults).toContain("origin_preference:MFN");
    expect(decision.grade).toBe("auto_pass");
  });

  it("L5-T-customs_classification-018 abstains on missing supplementary units and records unlockable evidence", () => {
    const measureScan = scanMeasures({
      uk10: "7604210000",
      measures: [
        {
          id: "specific-aluminium-kg",
          measureType: "specific_duty",
          dutyExpression: { kind: "specific", unit: "kg", amountPence: 80 },
        },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: noTradeRemedyExporterResolution("7604210000"),
    });
    const confidenceBreakdown = computeConfidenceBreakdown({
      exporterResolution: noTradeRemedyExporterResolution("7604210000"),
      vatResolution: standardVatResolution("7604210000"),
      measureScanResult: measureScan,
      marginSnapshot: {
        snapshotId: "supplementary-unit-abstain",
        netMarginBaseBps: 1_200,
        resolvedNetMarginBaseBps: {
          supplementary_units: 1_550,
        },
      },
    });

    expect(measureScan.selectedDutyPence).toBeNull();
    expect(measureScan.blockers).toContain(SUPPLEMENTARY_UNIT_MISSING);
    expect(measureScan.supplementaryUnitsRequiredJson).toEqual(["kg_net"]);
    expect(measureScan.dutyBreakdownJson).toMatchObject({
      components: [{ measureId: "specific-aluminium-kg", amountPence: null }],
    });
    expect(confidenceBreakdown.evidence_leverage).toContainEqual({
      parameter: "supplementary_units",
      defaultApplied: "abstained",
      evidenceRequired: ["supplementary_quantity:kg_net"],
      marginDeltaBps: 350,
      unlockable: true,
      netMarginBaseBps: 1_200,
      resolvedNetMarginBaseBps: 1_550,
      snapshotId: "supplementary-unit-abstain",
    });
  });

  it("L5-T-customs_classification-033 keeps open-question runtime behavior conservative and present-tense", () => {
    expect(openQuestionBehaviors()).toEqual([
      {
        question: "unresolved_logistics",
        runtimeBehavior: "run_boundary_scenarios",
        bidReadiness: "review_required",
      },
      {
        question: "chinese_sourcing_exporter",
        runtimeBehavior: "residual_trade_remedy_default",
        bidReadiness: "broker_required_when_remedy_active",
      },
      {
        question: "customs_spread",
        runtimeBehavior: "block_on_material_branch_divergence",
        bidReadiness: "broker_required",
      },
      {
        question: "packaging_subcategory",
        runtimeBehavior: "expand_all_plausible_hs6_before_shortlist",
        bidReadiness: "review_required_until_specific",
      },
      {
        question: "cbam_timing",
        runtimeBehavior: "flag_exposure_only_until_2027_secondary_legislation",
        bidReadiness: "review_required",
      },
    ]);
  });

  it("L5-T-customs_classification-036 classifies every landed_cost_estimates column as populated nullable or sync-preserved", async () => {
    const columns = await landedCostEstimateColumns();
    const accountedFor = new Set([
      ...L5_LANDED_COST_POPULATED_COLUMNS,
      ...L5_LANDED_COST_EXPLICIT_NULL_COLUMNS,
      ...L5_LANDED_COST_SYNC_COLUMNS,
    ]);

    expect(columns).toEqual(L5_LANDED_COST_COLUMN_CONTRACT);
    expect([...accountedFor].sort()).toEqual([...new Set(accountedFor)].sort());
    expect(
      columns.filter((column) => !accountedFor.has(column as LandedCostColumn)),
    ).toEqual([]);
    expect(L5_LANDED_COST_EXPLICIT_NULL_COLUMNS).toContain("cbam_estimate_gbp_pence");
  });
});

describe("customs classification accuracy harness", () => {
  it("L5-T-customs_classification-028 reports gold-tier labels separately and gates auto-classification safety", () => {
    const report = scoreCategory(
      repeatedAccuracyCases(120, { labelTier: "gold", casePrefix: "gold" }),
    );

    expect(report.labelTierCounts).toEqual({
      gold: 120,
      silver: 0,
      bronze: 0,
    });
    expect(report.productionGate).toBe("eligible");
    expect(report.wilsonLowerBounds.reviewTriggerRecallBps).toBeGreaterThanOrEqual(
      9_500,
    );
    expect(toAccuracyHarnessMetricContract(report)).toMatchObject({
      category: "fasteners",
      label_tier_counts: { gold: 120, silver: 0, bronze: 0 },
      duty_outcome_accuracy_bps: 10_000,
      production_gate: "eligible",
    });
  });

  it("L5-T-customs_classification-029 requires silver-tier independent reviewer reasoning and separates it from gold", () => {
    expect(() =>
      scoreCategory([
        accuracyCase({
          caseId: "silver-without-two-reviewers",
          labelTier: "silver",
          reviewers: [{ reviewerId: "reviewer-a", reasoning: "single pass" }],
        }),
      ]),
    ).toThrow("requires two independent reviewers");

    const report = scoreCategory(
      repeatedAccuracyCases(120, { labelTier: "silver", casePrefix: "silver" }),
    );

    expect(report.labelTierCounts).toEqual({
      gold: 0,
      silver: 120,
      bronze: 0,
    });
    expect(report.productionGate).toBe("eligible");
  });

  it("L5-T-customs_classification-030 prevents bronze-only labels from satisfying production auto-classification safety", () => {
    const report = scoreCategory(
      repeatedAccuracyCases(120, { labelTier: "bronze", casePrefix: "bronze" }),
    );

    expect(report.labelTierCounts).toEqual({
      gold: 0,
      silver: 0,
      bronze: 120,
    });
    expect(report.productionGate).toBe("ineligible_bronze_only");
  });

  it("L5-T-customs_classification-031 scores abstention as positive class and fails fabricated actionable estimates", () => {
    const report = scoreCategory([
      accuracyCase({
        caseId: "correct-broker-abstention",
        groundTruth: {
          expectedDecision: "broker_required",
          expectedDutyRange: { minPence: 80_000, maxPence: 120_000 },
        },
        systemOutput: {
          abstained: true,
          decisionGrade: "broker_required",
          dutyPence: 0,
        },
      }),
      accuracyCase({
        caseId: "fabricated-actionable-estimate",
        groundTruth: {
          expectedUk10: "7318158898",
          expectedDecision: "broker_required",
          expectedDutyRange: { minPence: 40_000, maxPence: 60_000 },
        },
        systemOutput: {
          selectedUk10: "3923100000",
          decisionGrade: "auto_pass",
          abstained: false,
          dutyPence: 50_000,
        },
        mutationFamily: "fabricated_evidence",
      }),
    ]);

    expect(report.correctAbstentionCount).toBe(1);
    expect(report.wrongCodeSuccessCount).toBe(1);
    expect(report.unsafeAutoClassificationRateBps).toBeGreaterThan(0);
    expect(report.productionGate).toBe("ineligible_catastrophic");
  });

  it("L5-T-customs_classification-032 exposes all eight accuracy measures plus catastrophic and unsafe auto-classification rates", () => {
    const report = scoreCategory([
      accuracyCase({ caseId: "clean-success" }),
      accuracyCase({
        caseId: "catastrophic-understatement",
        groundTruth: {
          expectedDecision: "broker_required",
          expectedDutyRange: { minPence: 120_000, maxPence: 150_000 },
          customsValuePence: 1_000_000,
          expectedTradeRemedy: true,
          expectedAdditionalCodeRequired: true,
          expectedSupplementaryUnitRequired: true,
        },
        systemOutput: {
          decisionGrade: "auto_pass",
          abstained: false,
          dutyPence: 10_000,
          tradeRemedyActive: false,
          additionalCodeRequired: false,
          supplementaryUnitRequired: false,
        },
        mutationFamily: "mutated",
      }),
    ]);

    expect(report.dutyOutcomeAccuracyBps).toBeLessThan(10_000);
    expect(report.tradeRemedyYesNoBps).toBeLessThan(10_000);
    expect(report.additionalCodeRequiredYesNoBps).toBeLessThan(10_000);
    expect(report.supplementaryUnitRequiredYesNoBps).toBeLessThan(10_000);
    expect(report.reviewTriggerRecallBps).toBeLessThan(10_000);
    expect(report.catastrophicUnderstatementRateBps).toBeGreaterThan(0);
    expect(report.unsafeAutoClassificationRateBps).toBeGreaterThan(0);
    expect(report.correctAbstentionCount).toBe(0);
  });

  it("L5-T-customs_classification-035 covers mutation taxonomy and promotion math helpers", () => {
    const taxonomyCases = CUSTOMS_ACCURACY_MUTATION_FAMILIES.map(
      (mutationFamily) =>
        accuracyCase({
          caseId: `taxonomy-${mutationFamily}`,
          mutationFamily,
          pairId: mutationFamily === "pairwise" ? "pair-001" : undefined,
        }),
    );
    const audit = auditMutationTaxonomy(taxonomyCases);

    expect(audit.missingFamilies).toEqual([]);
    expect(audit.requiredFamilies).toEqual(CUSTOMS_ACCURACY_MUTATION_FAMILIES);
    expect(audit.requiredCustomsMutationDimensions).toEqual(
      CUSTOMS_SPECIFIC_MUTATION_DIMENSIONS,
    );
    expect(audit.hasPairwiseCoverage).toBe(true);
    expect(audit.hasCrossModelAudit).toBe(true);
    expect(audit.hasOverrideFeedback).toBe(true);
    expect(audit.hasDemotionThresholdCalculation).toBe(true);
    expect(calculateWilsonLowerBoundBps(120, 120)).toBeGreaterThanOrEqual(9_500);
    expect(calculateRuleOfThreeUpperBoundBps(0, 100)).toBe(300);
    expect(
      calculateDemotionThresholdBps({
        catastrophicUnderstatementRateBps: 0,
        unsafeAutoClassificationRateBps: 0,
      }),
    ).toBe(300);
  });
});

describe("customs classification gold-set corpora", () => {
  it("L5-T-customs_classification-027 verifies the packaging-cartons calibration corpus", async () => {
    const requiredRootFiles = [
      "canary.inputs.jsonl",
      "golden.assertions.yaml",
      "metadata.json",
      "manifest.md",
    ];
    const variantDirectories = ["clean", "weird", "mutated", "pairwise"];

    for (const fileName of requiredRootFiles) {
      expect(
        await fileExists(new URL(fileName, packagingCartonsCorpusRootUrl)),
        fileName,
      ).toBe(true);
    }
    for (const variant of variantDirectories) {
      expect(
        await fileExists(
          new URL(`${variant}/manifest.md`, packagingCartonsCorpusRootUrl),
        ),
        `${variant}/manifest.md`,
      ).toBe(true);
      const metadata = await readJson<CalibrationVariantMetadata>(
        new URL(`${variant}/metadata.json`, packagingCartonsCorpusRootUrl),
      );
      expect(metadata).toMatchObject({
        category: "packaging-cartons",
        corpus_kind: variant,
        fixture_surface: "calibration-corpus",
      });
    }

    const [manifest, corpusMetadata, goldenAssertions, canaryRows] = await Promise.all([
      readText(new URL("manifest.md", packagingCartonsCorpusRootUrl)),
      readJson<PackagingCartonsMetadata>(
        new URL("metadata.json", packagingCartonsCorpusRootUrl),
      ),
      readText(
        new URL("golden.assertions.yaml", packagingCartonsCorpusRootUrl),
      ),
      readJsonl<PackagingCartonsCanaryRow>(
        new URL("canary.inputs.jsonl", packagingCartonsCorpusRootUrl),
      ),
    ]);

    expect(corpusMetadata).toMatchObject({
      category: "packaging-cartons",
      fixture_surface: "calibration-corpus",
      trade_remedy_active: false,
      minimum_corpus: {
        gold: 10,
        silver: 15,
        bronze: 10,
        pairwise_pairs: 12,
      },
    });
    expect(corpusMetadata.explicitly_excluded).toEqual([
      "aluminium_foil_packaging",
      "glass_packaging",
      "tin_mill_packaging",
      "metal_cans",
      "composite_packaging",
    ]);
    for (const excluded of corpusMetadata.explicitly_excluded) {
      expect(manifest).toContain(excluded);
    }
    expect(manifest).toContain('fixture_surface: "calibration-corpus"');
    expect(goldenAssertions).toContain("production_gate: eligible");
    expect(goldenAssertions).toContain("dangerous_subfamilies_route_to_broker");

    expect(new Set(canaryRows.map((row) => row.case_id)).size).toBe(
      canaryRows.length,
    );
    expect(
      canaryRows.every((row) => row.category === "packaging-cartons"),
    ).toBe(true);
    expect(countCorpusRows(canaryRows, (row) => row.label_tier === "gold")).toBeGreaterThanOrEqual(10);
    expect(countCorpusRows(canaryRows, (row) => row.label_tier === "silver")).toBeGreaterThanOrEqual(15);
    expect(countCorpusRows(canaryRows, (row) => row.label_tier === "bronze")).toBeGreaterThanOrEqual(10);
    expect(
      countCorpusRows(canaryRows, (row) => row.expected_uk10.startsWith("4819")),
    ).toBeGreaterThan(0);
    expect(
      countCorpusRows(canaryRows, (row) => row.expected_uk10.startsWith("4823")),
    ).toBeGreaterThan(0);
    expect(
      countCorpusRows(canaryRows, (row) => row.rejected_hs6 === "3923"),
    ).toBeGreaterThan(0);
    expect(
      countCorpusRows(canaryRows, (row) => row.dangerous_subfamily),
    ).toBe(0);

    const pairwiseRowsById = groupPairwiseRows(canaryRows);
    expect(pairwiseRowsById.size).toBeGreaterThanOrEqual(12);
    for (const rows of pairwiseRowsById.values()) {
      expect(rows).toHaveLength(2);
      expect(new Set(rows.map((row) => row.pair_dimension))).toHaveLength(1);
    }

    const mutationDimensions = new Set(
      canaryRows
        .map((row) => row.mutation_dimension)
        .filter((value): value is string => typeof value === "string"),
    );
    expect(mutationDimensions.size).toBeGreaterThanOrEqual(5);
    expect([...mutationDimensions]).toEqual(
      expect.arrayContaining([
        "hs6_perturbation",
        "material_perturbation",
        "currency_swap",
        "supplementary_unit_removal",
        "incoterm_swap",
      ]),
    );

    const report = scoreCategory(
      canaryRows.map((row) =>
        accuracyCase({
          caseId: row.case_id,
          category: "packaging-cartons",
          labelTier: row.label_tier,
          mutationFamily: row.mutation_family,
          pairId: row.pair_id,
          groundTruth: {
            expectedUk10: row.expected_uk10,
            expectedDutyRange: { minPence: 3_000, maxPence: 5_000 },
            expectedTradeRemedy: false,
            expectedAdditionalCodeRequired: false,
            expectedSupplementaryUnitRequired:
              row.expected_supplementary_unit_required,
            expectedDecision: "auto_pass",
          },
          systemOutput: {
            selectedUk10: row.expected_uk10,
            dutyPence: 4_000,
            tradeRemedyActive: false,
            additionalCodeRequired: false,
            supplementaryUnitRequired:
              row.expected_supplementary_unit_required,
            decisionGrade: "auto_pass",
            abstained: false,
          },
        }),
      ),
    );
    expect(report.productionGate).toBe("eligible");
    expect(report.category).toBe("packaging-cartons");
  });
});

describe("customs classification exporter defaults", () => {
  it("L5-T-customs_classification-013 applies MFN when origin proof is missing and records origin evidence leverage", () => {
    const measureScanResult = scanMeasures({
      uk10: "4819100000",
      measures: [
        {
          id: "mfn-paperboard",
          measureType: "MFN",
          dutyExpression: { kind: "ad_valorem", rateBps: 650 },
        },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: noTradeRemedyExporterResolution("4819100000"),
    });
    const confidenceBreakdown = computeConfidenceBreakdown({
      originPreference: {
        originProofAvailable: false,
        evidenceRequired: ["origin_certificate"],
      },
      exporterResolution: noTradeRemedyExporterResolution("4819100000"),
      vatResolution: standardVatResolution("4819100000"),
      measureScanResult,
      marginSnapshot: {
        snapshotId: "origin-default-same-quote-freight-fx",
        netMarginBaseBps: 1_800,
        resolvedNetMarginBaseBps: {
          origin_preference: 2_450,
        },
      },
    });

    expect(measureScanResult.selectedDutyPence).toBe(65_000);
    expect(confidenceBreakdown.active_defaults).toContain(
      "origin_preference:MFN",
    );
    expect(confidenceBreakdown.origin_preference).toMatchObject({
      default_applied: true,
      default_applied_value: "MFN",
      evidence_required: ["origin_certificate"],
      margin_delta_bps: 650,
    });
    expect(confidenceBreakdown.evidence_leverage).toEqual([
      {
        parameter: "origin_preference",
        defaultApplied: "MFN",
        evidenceRequired: ["origin_certificate"],
        marginDeltaBps: 650,
        unlockable: true,
        netMarginBaseBps: 1_800,
        resolvedNetMarginBaseBps: 2_450,
        snapshotId: "origin-default-same-quote-freight-fx",
      },
    ]);
  });

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

  it("L5-T-customs_classification-015 applies out-of-quota rate and records quota allocation proof leverage", () => {
    const measureScanResult = scanMeasures({
      uk10: "7318158898",
      measures: [
        {
          id: "quota-fasteners",
          measureType: "TRQ",
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
      exporterResolution: noTradeRemedyExporterResolution("7318158898"),
    });
    const confidenceBreakdown = computeConfidenceBreakdown({
      exporterResolution: noTradeRemedyExporterResolution("7318158898"),
      vatResolution: standardVatResolution("7318158898"),
      measureScanResult,
      marginSnapshot: {
        snapshotId: "quota-default-same-quote-freight-fx",
        netMarginBaseBps: 1_000,
        resolvedNetMarginBaseBps: {
          quota_allocation: 2_200,
        },
      },
    });

    expect(measureScanResult.selectedDutyPence).toBe(120_000);
    expect(measureScanResult.additionalCodesRequirementsJson).toEqual([
      {
        code: "UKQ-FASTENERS",
        satisfiability: "unknown",
        evidenceRequired: ["quota_allocation_proof"],
      },
    ]);
    expect(confidenceBreakdown.quota_allocation).toMatchObject({
      default_applied: true,
      default_applied_value: "out_of_quota",
      evidence_required: ["quota_allocation_proof"],
      margin_delta_bps: 1_200,
    });
    expect(confidenceBreakdown.evidence_leverage).toContainEqual({
      parameter: "quota_allocation",
      defaultApplied: "out_of_quota",
      evidenceRequired: ["quota_allocation_proof"],
      marginDeltaBps: 1_200,
      unlockable: true,
      netMarginBaseBps: 1_000,
      resolvedNetMarginBaseBps: 2_200,
      snapshotId: "quota-default-same-quote-freight-fx",
    });
  });

  it("L5-T-customs_classification-017 records Incoterm boundary scenarios as conservative logistics evidence", () => {
    const measureScanResult = scanMeasures({
      uk10: "4819100000",
      measures: [
        {
          id: "mfn-paperboard",
          measureType: "MFN",
          dutyExpression: { kind: "ad_valorem", rateBps: 0 },
        },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution: noTradeRemedyExporterResolution("4819100000"),
    });
    const freightOutput = incotermBoundaryFreightOutput();
    const confidenceBreakdown = computeConfidenceBreakdown({
      exporterResolution: noTradeRemedyExporterResolution("4819100000"),
      vatResolution: standardVatResolution("4819100000"),
      measureScanResult,
      freightOutput,
      marginSnapshot: {
        snapshotId: "incoterm-default-same-quote-freight-fx",
        netMarginBaseBps: 1_500,
        resolvedNetMarginBaseBps: {
          incoterm: 1_900,
        },
      },
    });

    expect(freightOutput.boundaryScenariosRun.map((scenario) => scenario.incoterm)).toEqual([
      "EXW",
      "FOB",
      "CIF",
    ]);
    expect(freightOutput.selectedIncoterm).toBe("EXW");
    expect(freightOutput.selectedEstimate?.baseGbpPence).toBe(250_000);
    expect(confidenceBreakdown.incoterm).toMatchObject({
      default_applied: true,
      default_applied_value: "boundary_scenarios",
      evidence_required: ["incoterm"],
      margin_delta_bps: 400,
    });
  });

  it("L5-T-customs_classification-019 computes evidence leverage deltas against the same quote, freight, and FX snapshot", () => {
    const exporterResolution = resolveExporter({
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
    const measureScanResult = scanMeasures({
      uk10: "7604210000",
      measures: [
        {
          id: "quota-aluminium",
          measureType: "TRQ",
          dutyExpression: { kind: "ad_valorem", rateBps: 0 },
          conditions: {
            quotaCode: "UKQ-ALUMINIUM",
            quotaProofAvailable: false,
            inQuotaDutyExpression: { kind: "ad_valorem", rateBps: 0 },
            outOfQuotaDutyExpression: { kind: "ad_valorem", rateBps: 1_000 },
            evidenceRequired: ["quota_allocation_proof"],
          },
        },
        {
          id: "specific-aluminium-kg",
          measureType: "specific_duty",
          dutyExpression: { kind: "specific", unit: "kg", amountPence: 80 },
        },
      ],
      customsValuePence: 1_000_000,
      supplementaryQuantities: {},
      exporterResolution,
    });
    const confidenceBreakdown = computeConfidenceBreakdown({
      originPreference: { originProofAvailable: false },
      exporterResolution,
      vatResolution: resolveVat({
        uk10: "7604210000",
        measures: [],
        reliefClaims: [{ kind: "VATZ", evidenceArtifactPaths: [] }],
      }),
      measureScanResult,
      freightOutput: incotermBoundaryFreightOutput(),
      marginSnapshot: {
        snapshotId: "same-quote-freight-fx-snapshot",
        netMarginBaseBps: 1_000,
        resolvedNetMarginBaseBps: {
          origin_preference: 1_300,
          exporter_rate: 1_000,
          quota_allocation: 1_250,
          vat_rate: 1_200,
          incoterm: 1_100,
          supplementary_units: 1_000,
        },
      },
    });

    expect(confidenceBreakdown.active_defaults).toEqual([
      "exporter_rate:residual_default",
      "incoterm:boundary_scenarios",
      "origin_preference:MFN",
      "quota_allocation:out_of_quota",
      "supplementary_units:abstained",
      "vat_rate:standard_rate_default",
    ]);
    expect(
      confidenceBreakdown.evidence_leverage.map((item) => item.parameter),
    ).toEqual([
      "incoterm",
      "origin_preference",
      "quota_allocation",
      "supplementary_units",
      "vat_rate",
    ]);
    expect(
      confidenceBreakdown.evidence_leverage.every(
        (item) =>
          item.snapshotId === "same-quote-freight-fx-snapshot" &&
          item.marginDeltaBps ===
            item.resolvedNetMarginBaseBps - item.netMarginBaseBps,
      ),
    ).toBe(true);
    expect(confidenceBreakdown.evidence_leverage).not.toContainEqual(
      expect.objectContaining({ parameter: "exporter_rate" }),
    );
    expect(confidenceBreakdown.supplementary_units).toMatchObject({
      default_applied: true,
      default_applied_value: "abstained",
      evidence_required: ["supplementary_quantity:kg_net"],
      margin_delta_bps: 0,
    });
    expect(confidenceBreakdown.unlockable_margin_bps).toBe(850);
  });
});

describe("customs classification divergence readiness", () => {
  it("L5-T-customs_classification-020 treats material tariff branch divergence as a hard blocker without averaging", () => {
    const remedyDivergence = detectBranchDivergence([
      branchCandidate({
        uk10Code: "7318158898",
        parentHs6: "7318",
        rank: 1,
        dutyOutcomePence: 120_000,
        dutyType: "ad_valorem",
        hasTradeRemedy: false,
      }),
      branchCandidate({
        uk10Code: "7318159599",
        parentHs6: "7318",
        rank: 2,
        dutyOutcomePence: 420_000,
        dutyType: "ad_valorem",
        hasTradeRemedy: true,
      }),
    ]);

    expect(remedyDivergence).toMatchObject({
      divergent: true,
      divergenceKind: "remedy_no_remedy",
      selectedBranchAuto: null,
      worstCaseDutyPence: 420_000,
      bestCaseDutyPence: 120_000,
    });
    expect(remedyDivergence.emittedCodes).toContain(TARIFF_BRANCH_DIVERGENCE);
    expect(
      (remedyDivergence as { averagedDutyPence?: number }).averagedDutyPence,
    ).toBeUndefined();

    const familyDivergence = detectBranchDivergence([
      branchCandidate({
        uk10Code: "4819100000",
        parentHs6: "4819",
        dutyOutcomePence: 40_000,
      }),
      branchCandidate({
        uk10Code: "3923100000",
        parentHs6: "3923",
        dutyOutcomePence: 140_000,
      }),
    ]);

    expect(familyDivergence.divergent).toBe(true);
    expect(familyDivergence.divergenceKind).toBe("material_spread");
    expect(familyDivergence.emittedCodes).toEqual([
      CLASSIFICATION_AMBIGUOUS,
      TARIFF_BRANCH_DIVERGENCE,
    ]);
  });

  it("L5-T-customs_classification-021 auto-passes only when divergence, measures, freshness, and net gate coverage are clean", () => {
    const divergence = detectBranchDivergence([
      branchCandidate({
        uk10Code: "4819100000",
        parentHs6: "4819",
        dutyOutcomePence: 60_000,
      }),
      branchCandidate({
        uk10Code: "4819200000",
        parentHs6: "4819",
        rank: 2,
        dutyOutcomePence: 62_000,
      }),
    ]);
    const result = classifyDecisionGrade({
      ...baseDecisionGradeInput(),
      divergence,
      customsCostSpreadBps: divergence.customsCostSpreadBps,
    });

    expect(divergence.selectedBranchAuto?.uk10Code).toBe("4819100000");
    expect(result).toEqual({
      grade: "auto_pass",
      blockers: [],
      warnings: [],
    });

    const marginReview = classifyDecisionGrade({
      ...baseDecisionGradeInput(),
      worstCaseNetMarginBps: 1_099,
      gateBps: 1_100,
    });

    expect(marginReview).toMatchObject({
      grade: "review_required",
      blockers: [],
      warnings: ["worst_case_net_margin_below_gate"],
    });
  });

  it("L5-T-customs_classification-034 produces every L5-to-L6 readiness blocker and blocks floor-only quotes without override", () => {
    const ambiguousDivergence = detectBranchDivergence([
      branchCandidate({
        uk10Code: "4819100000",
        parentHs6: "4819",
        dutyOutcomePence: 40_000,
      }),
      branchCandidate({
        uk10Code: "3923100000",
        parentHs6: "3923",
        rank: 2,
        dutyOutcomePence: 140_000,
      }),
    ]);
    const ambiguous = classifyDecisionGrade({
      ...baseDecisionGradeInput(),
      divergence: ambiguousDivergence,
      customsCostSpreadBps: ambiguousDivergence.customsCostSpreadBps,
      netGateCoverage: { allBranchesSameFamily: false },
    });

    expect(ambiguous.grade).toBe("broker_required");
    expect(ambiguous.blockers).toEqual(
      expect.arrayContaining([
        CLASSIFICATION_AMBIGUOUS,
        TARIFF_BRANCH_DIVERGENCE,
      ]),
    );

    const supplementaryUnitMissing = classifyDecisionGrade({
      ...baseDecisionGradeInput(),
      measureScan: scanMeasures({
        uk10: "7604210000",
        measures: [
          {
            id: "specific-aluminium-kg",
            measureType: "specific_duty",
            dutyExpression: { kind: "specific", unit: "kg", amountPence: 80 },
          },
        ],
        customsValuePence: 1_000_000,
        supplementaryQuantities: {},
        exporterResolution: noTradeRemedyExporterResolution("7604210000"),
      }),
    });

    expect(supplementaryUnitMissing.grade).toBe("broker_required");
    expect(supplementaryUnitMissing.blockers).toContain(
      "SUPPLEMENTARY_UNIT_MISSING",
    );

    const residualExporter = classifyDecisionGrade({
      ...baseDecisionGradeInput(),
      exporterRateBasis: "residual_default",
      activeTradeRemedy: true,
    });

    expect(residualExporter.grade).toBe("broker_required");
    expect(residualExporter.blockers).toContain(EXPORTER_UNRESOLVED_BLOCKER);

    const expiredFreshness = classifyDecisionGrade({
      ...baseDecisionGradeInput(),
      freshnessSweep: decisionFreshnessSweep({
        measuresFreshUntil: "2026-05-19T00:00:00Z",
      }),
    });

    expect(expiredFreshness.grade).toBe("broker_required");
    expect(expiredFreshness.blockers).toContain(
      CLASSIFICATION_FRESH_UNTIL_EXPIRED,
    );

    const floorOnlyQuotes = classifyDecisionGrade({
      ...baseDecisionGradeInput(),
      quoteRoles: ["optimistic_floor", "domestic_wholesale_floor"],
    });

    expect(floorOnlyQuotes.grade).toBe("broker_required");
    expect(floorOnlyQuotes.blockers).toContain(FLOOR_ONLY_QUOTE_ROLE);

    const operatorOverride = classifyDecisionGrade({
      ...baseDecisionGradeInput(),
      quoteRoles: ["optimistic_floor", "domestic_wholesale_floor"],
      operatorOverrideFloorQuoteRole: true,
    });

    expect(operatorOverride).toMatchObject({
      grade: "auto_pass",
      blockers: [],
    });
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

function noTradeRemedyExporterResolution(commodityCode: string) {
  return resolveExporter({
    commodityCode,
    countryOfOrigin: "CN",
    supplierQuotes: [],
    measuresAtCommodity: [],
  });
}

function standardVatResolution(uk10: string) {
  return resolveVat({
    uk10,
    measures: [
      {
        id: "vat-standard",
        measureType: "import_vat",
        dutyExpression: { kind: "ad_valorem" as const, rateBps: 2_000 },
      },
    ],
    reliefClaims: [],
  });
}

function incotermBoundaryFreightOutput(): FreightEngineOutputs {
  return {
    selectedMode: "sea_lcl",
    selectedEstimate: {
      provider: "freightos",
      mode: "sea_lcl",
      lowGbpPence: 230_000,
      baseGbpPence: 250_000,
      highGbpPence: 280_000,
      checkedAt: "2026-05-19T12:00:00Z",
    },
    selectedIncoterm: "EXW",
    selectedPackagingState: "palletized",
    boundaryScenariosRun: [
      freightBoundaryScenario("EXW", 250_000),
      freightBoundaryScenario("FOB", 180_000),
      freightBoundaryScenario("CIF", 150_000),
    ],
    outcomeKind: "agreed",
    warningsAndBlockers: [],
    manualEstimateRequired: false,
    missingLogisticsFacts: ["incoterm"],
    unlockableMarginNote: "Clarify incoterm to replace conservative freight boundary scenarios.",
  };
}

function freightBoundaryScenario(
  incoterm: "EXW" | "FOB" | "CIF",
  baseGbpPence: number,
): FreightEngineOutputs["boundaryScenariosRun"][number] {
  return {
    scenario: `${incoterm}:palletized`,
    incoterm,
    packagingState: "palletized",
    selectedMode: "sea_lcl",
    estimate: {
      provider: incoterm === "CIF" ? "searates" : "freightos",
      mode: "sea_lcl",
      lowGbpPence: baseGbpPence - 20_000,
      baseGbpPence,
      highGbpPence: baseGbpPence + 30_000,
      checkedAt: "2026-05-19T12:00:00Z",
    },
    outcomeKind: "agreed",
    warningsAndBlockers: [],
    missingLogisticsFacts: [],
  };
}

function branchCandidate(
  overrides: Partial<TenDigitCandidate> = {},
): TenDigitCandidate {
  return {
    uk10Code: "4819100000",
    parentHs6: "4819",
    rank: 1,
    scoreBps: 9_000,
    dutyOutcomePence: 60_000,
    dutyType: "ad_valorem",
    hasTradeRemedy: false,
    slotCompatibility: {
      descriptionMatchBps: 10_000,
      materialMatchBps: 10_000,
      unitCompatibility: "ok",
      exclusionsTriggered: [],
      scopeNoteFlags: [],
    },
    candidateRecallAgreement: [],
    ...overrides,
  };
}

function cleanMeasureScan() {
  return scanMeasures({
    uk10: "4819100000",
    measures: [
      {
        id: "mfn-paperboard",
        measureType: "MFN",
        dutyExpression: { kind: "ad_valorem", rateBps: 0 },
      },
    ],
    customsValuePence: 1_000_000,
    supplementaryQuantities: {},
    exporterResolution: noTradeRemedyExporterResolution("4819100000"),
  });
}

function baseDecisionGradeInput(): DecisionGradeInput {
  const divergence = detectBranchDivergence([branchCandidate()]);
  return {
    divergence,
    measureScan: cleanMeasureScan(),
    exporterRateBasis: "named_exporter",
    freshnessSweep: decisionFreshnessSweep(),
    worstCaseNetMarginBps: 1_600,
    gateBps: 1_100,
    customsCostSpreadBps: divergence.customsCostSpreadBps,
    netGateCoverage: { allBranchesSameFamily: true },
    activeTradeRemedy: false,
    quoteRoles: ["benchmark_ceiling"],
  };
}

function decisionFreshnessSweep(
  overrides: Partial<Parameters<typeof sweepFreshness>[0]["vectors"]> = {},
) {
  return sweepFreshness({
    uk10: "4819100000",
    vectors: freshnessVectors(overrides),
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
  });
}

async function readText(url: URL): Promise<string> {
  return await Bun.file(url).text();
}

async function fileExists(url: URL): Promise<boolean> {
  return await Bun.file(url).exists();
}

async function readJson<T>(url: URL): Promise<T> {
  return parseJsonFixture<T>(await readText(url), url.pathname);
}

async function readJsonl<T>(url: URL): Promise<T[]> {
  const text = await readText(url);
  return text
    .trim()
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line, index) =>
      parseJsonFixture<T>(line, `${url.pathname}:${index + 1}`),
    );
}

function parseJsonFixture<T>(text: string, label: string): T {
  try {
    return JSON.parse(text) as T;
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to parse JSON fixture ${label}: ${reason}`);
  }
}

async function landedCostEstimateColumns(): Promise<LandedCostColumn[]> {
  const sqliteMigration = await readText(sqliteMigrationUrl);
  const tableMatch = sqliteMigration.match(
    /CREATE TABLE IF NOT EXISTS landed_cost_estimates \((?<body>[\s\S]*?)\n\);/,
  );
  expect(tableMatch?.groups?.body).toBeTruthy();
  return ((tableMatch?.groups?.body ?? "")
    .split("\n")
    .map((line) => line.trim().replace(/,$/, ""))
    .filter((line) => line.length > 0)
    .map((line) => line.split(/\s+/, 1)[0])
    .filter((column) =>
      !["PRIMARY", "FOREIGN", "CHECK", "UNIQUE"].includes(column),
    )) as LandedCostColumn[];
}

type LandedCostColumn = (typeof L5_LANDED_COST_COLUMN_CONTRACT)[number];

type CalibrationVariantMetadata = {
  category: string;
  corpus_kind: string;
  fixture_surface: string;
};

type PackagingCartonsMetadata = {
  category: "packaging-cartons";
  fixture_surface: "calibration-corpus";
  trade_remedy_active: boolean;
  explicitly_excluded: string[];
  minimum_corpus: {
    gold: number;
    silver: number;
    bronze: number;
    pairwise_pairs: number;
  };
};

type PackagingCartonsCanaryRow = {
  case_id: string;
  category: "packaging-cartons";
  label_tier: AccuracyCase["labelTier"];
  mutation_family: AccuracyCase["mutationFamily"];
  expected_uk10: string;
  rejected_hs6?: string;
  dangerous_subfamily: boolean;
  expected_supplementary_unit_required: boolean;
  mutation_dimension?: string;
  pair_id?: string;
  pair_dimension?: string;
};

function countCorpusRows(
  rows: PackagingCartonsCanaryRow[],
  predicate: (row: PackagingCartonsCanaryRow) => boolean,
): number {
  return rows.filter(predicate).length;
}

function groupPairwiseRows(
  rows: PackagingCartonsCanaryRow[],
): Map<string, PackagingCartonsCanaryRow[]> {
  const grouped = new Map<string, PackagingCartonsCanaryRow[]>();
  for (const row of rows) {
    if (row.mutation_family !== "pairwise" || typeof row.pair_id !== "string") {
      continue;
    }
    grouped.set(row.pair_id, [...(grouped.get(row.pair_id) ?? []), row]);
  }
  return grouped;
}

const L5_LANDED_COST_COLUMN_CONTRACT = [
  "id",
  "process_id",
  "notice_id",
  "product_candidate_id",
  "supplier_quote_id",
  "shipping_mode",
  "mode_confidence_bps",
  "packing_scenario",
  "declarable_commodity_code_10",
  "candidate_commodity_codes_json",
  "additional_codes",
  "additional_code_requirements_json",
  "supplementary_units_required_json",
  "commodity_confidence_bps",
  "classification_stability_bps",
  "customs_valuation_method",
  "valuation_mode",
  "customs_fx_rate",
  "commercial_fx_rate",
  "fx_basis_delta_bps",
  "fx_rate_checked_at",
  "supplier_cost_gbp_pence",
  "freight_low_gbp_pence",
  "freight_base_gbp_pence",
  "freight_high_gbp_pence",
  "customs_duty_gbp_pence",
  "trade_remedy_duty_gbp_pence",
  "duty_outcome_min_gbp_pence",
  "duty_outcome_max_gbp_pence",
  "customs_cost_spread_bps",
  "trade_remedy_state",
  "trade_remedy_cases_json",
  "duty_breakdown_json",
  "measure_interaction_json",
  "exporter_rate_basis",
  "exporter_identity_confidence_bps",
  "origin_evidence_state",
  "import_vat_gbp_pence",
  "vat_rate_state",
  "import_vat_treatment",
  "vat_recovery_rate_bps",
  "uk_final_mile_cost_gbp_pence",
  "other_fees_gbp_pence",
  "economic_landed_cost_gbp_pence",
  "cash_outlay_gbp_pence",
  "cash_to_clear_gbp_pence",
  "working_capital_cost_gbp_pence",
  "revenue_basis_type",
  "revenue_confidence_bps",
  "customs_resolution_confidence_bps",
  "commercial_estimate_confidence_bps",
  "net_margin_low_bps",
  "net_margin_base_bps",
  "net_margin_high_bps",
  "decision_grade",
  "confidence_bps",
  "confidence_breakdown",
  "warnings",
  "blockers",
  "tariff_checked_at",
  "measures_fresh_until",
  "classification_fresh_until",
  "trade_remedy_watch_state",
  "cbam_exposure_flag",
  "cbam_estimate_gbp_pence",
  "owner_id",
  "row_version",
  "last_local_seq",
  "stage_run_id",
  "updated_at",
  "created_at",
] as const;

const L5_LANDED_COST_EXPLICIT_NULL_COLUMNS: LandedCostColumn[] = [
  "packing_scenario",
  "trade_remedy_duty_gbp_pence",
  "trade_remedy_cases_json",
  "working_capital_cost_gbp_pence",
  "cbam_estimate_gbp_pence",
];

const L5_LANDED_COST_SYNC_COLUMNS: LandedCostColumn[] = [
  "owner_id",
  "row_version",
  "last_local_seq",
  "stage_run_id",
  "updated_at",
  "created_at",
];

const L5_LANDED_COST_POPULATED_COLUMNS: LandedCostColumn[] =
  L5_LANDED_COST_COLUMN_CONTRACT.filter(
    (column) =>
      !L5_LANDED_COST_EXPLICIT_NULL_COLUMNS.includes(column) &&
      !L5_LANDED_COST_SYNC_COLUMNS.includes(column),
  );

type DecisionGradeCategoryBundle = {
  category: MarketplaceCategory;
  customs: {
    declarableCommodityCode10: string;
    dutyBreakdownJson: { components: Array<{ measureId: string }> };
  };
  fx: {
    customsFxRateCheckedAt: string;
  };
  freight: {
    providerEvidence: "freightos" | "searates";
  };
  vat: {
    importVatTreatment: string;
  };
  confidenceBreakdown: {
    evidence_leverage: Array<{ parameter: string; marginDeltaBps: number }>;
  };
  blockers: string[];
  decisionGrade: "auto_pass" | "review_required" | "broker_required";
};

function decisionGradeCategoryBundles(): DecisionGradeCategoryBundle[] {
  return [
    decisionGradeCategoryBundle("fasteners", "7318158898", "freightos"),
    decisionGradeCategoryBundle("packaging-cartons", "4819100000", "searates"),
    decisionGradeCategoryBundle("examination-gloves", "4015120000", "freightos"),
  ];
}

function decisionGradeCategoryBundle(
  category: MarketplaceCategory,
  commodityCode: string,
  providerEvidence: "freightos" | "searates",
): DecisionGradeCategoryBundle {
  return {
    category,
    customs: {
      declarableCommodityCode10: commodityCode,
      dutyBreakdownJson: { components: [{ measureId: `mfn-${commodityCode}` }] },
    },
    fx: { customsFxRateCheckedAt: "2026-05-19T12:00:00Z" },
    freight: { providerEvidence },
    vat: { importVatTreatment: "standard_rate_default" },
    confidenceBreakdown: {
      evidence_leverage: [{ parameter: "origin_preference", marginDeltaBps: 250 }],
    },
    blockers: [],
    decisionGrade: "review_required",
  };
}

type CategoryGateResult = {
  category: MarketplaceCategory;
  productionReady: boolean;
  blockers: string[];
};

function categoryGate(
  category: MarketplaceCategory,
  evidence: { corpusExists: boolean; goldLabels: number; silverLabels: number },
): CategoryGateResult {
  const blockers: string[] = [];
  if (!evidence.corpusExists) {
    blockers.push("customs_classification_corpus_missing");
  }
  if (evidence.goldLabels < 10 && evidence.silverLabels < 20) {
    blockers.push("evidence_tier_metrics_insufficient");
  }
  return {
    category,
    productionReady: blockers.length === 0,
    blockers,
  };
}

function openQuestionBehaviors() {
  return [
    {
      question: "unresolved_logistics",
      runtimeBehavior: "run_boundary_scenarios",
      bidReadiness: "review_required",
    },
    {
      question: "chinese_sourcing_exporter",
      runtimeBehavior: "residual_trade_remedy_default",
      bidReadiness: "broker_required_when_remedy_active",
    },
    {
      question: "customs_spread",
      runtimeBehavior: "block_on_material_branch_divergence",
      bidReadiness: "broker_required",
    },
    {
      question: "packaging_subcategory",
      runtimeBehavior: "expand_all_plausible_hs6_before_shortlist",
      bidReadiness: "review_required_until_specific",
    },
    {
      question: "cbam_timing",
      runtimeBehavior: "flag_exposure_only_until_2027_secondary_legislation",
      bidReadiness: "review_required",
    },
  ] as const;
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

type AccuracyCaseOverrides = Partial<
  Omit<AccuracyCase, "groundTruth" | "systemOutput">
> & {
  groundTruth?: Partial<AccuracyCase["groundTruth"]>;
  systemOutput?: Partial<AccuracyCase["systemOutput"]>;
};

function repeatedAccuracyCases(
  count: number,
  options: { labelTier: AccuracyCase["labelTier"]; casePrefix: string },
): AccuracyCase[] {
  return Array.from({ length: count }, (_, index) =>
    accuracyCase({
      caseId: `${options.casePrefix}-${index + 1}`,
      labelTier: options.labelTier,
    }),
  );
}

function accuracyCase(overrides: AccuracyCaseOverrides = {}): AccuracyCase {
  const labelTier = overrides.labelTier ?? "gold";
  return {
    caseId: overrides.caseId ?? "accuracy-case-001",
    category: overrides.category ?? "fasteners",
    labelTier,
    reviewers:
      overrides.reviewers ??
      (labelTier === "silver"
        ? [
            {
              reviewerId: "reviewer-a",
              reasoning: "HMRC family and measure evidence reviewed.",
            },
            {
              reviewerId: "reviewer-b",
              reasoning: "Broker-style classification reasoning cross-checked.",
            },
          ]
        : undefined),
    groundTruth: {
      expectedUk10: "7318158898",
      expectedDutyRange: { minPence: 90_000, maxPence: 110_000 },
      customsValuePence: 1_000_000,
      expectedTradeRemedy: false,
      expectedAdditionalCodeRequired: false,
      expectedSupplementaryUnitRequired: false,
      expectedDecision: "auto_pass",
      ...overrides.groundTruth,
    },
    systemOutput: {
      selectedUk10: "7318158898",
      dutyPence: 100_000,
      tradeRemedyActive: false,
      additionalCodeRequired: false,
      supplementaryUnitRequired: false,
      decisionGrade: "auto_pass",
      abstained: false,
      ...overrides.systemOutput,
    },
    mutationFamily: overrides.mutationFamily ?? "clean",
    pairId: overrides.pairId,
  };
}
