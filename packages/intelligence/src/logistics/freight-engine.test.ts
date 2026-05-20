import { describe, expect, it } from "bun:test";

import type { FreightEstimate, ModeSelectionFacts, ShippingMode } from "./freight-types";
import {
  FREIGHT_PROVIDER_DISAGREEMENT_THRESHOLD_BPS,
  FreightEngine,
  estimateWithDualProviders,
  reconcile,
} from "./freight-engine";
import {
  FREIGHTOS_SKILL_ID,
  createFreightosAdapter,
  freightosUsesPaidApiCredentials,
  normalizeFreightosSkillResult,
} from "./freightos-skill-adapter";
import { selectMode } from "./mode-selector";
import {
  SEARATES_SKILL_ID,
  createSearatesAdapter,
  normalizeSearatesSkillResult,
  searatesUsesPaidApiCredentials,
} from "./searates-skill-adapter";

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

describe("Freightos skill adapter", () => {
  it("L5-T-freight_engine-003 normalizes captured Freightos CDP output into GBP quote bands without paid API credentials", async () => {
    const adapter = createFreightosAdapter();
    const estimate = await adapter.estimate({
      mode: "sea_lcl",
      originPort: "CNTSN",
      destinationPort: "GBSOU",
      weightKg: 850,
      volumeM3: 5.4,
      cassettePath: "canary-01-dgu-tianjin-portsmouth-result.json",
      checkedAt: "2026-05-19T00:00:00.000Z",
    });

    const eurToGbp = 1 / 1.146;
    expect(FREIGHTOS_SKILL_ID).toBe("landed-cost/freightos-freight-estimate");
    expect(freightosUsesPaidApiCredentials()).toBe(false);
    expect(estimate.provider).toBe("freightos");
    expect(estimate.mode).toBe("sea_lcl");
    expect(estimate.failureCode).toBeUndefined();
    expect(estimate.baseGbpPence).toBe(Math.round(327_714 * eurToGbp));
    expect(estimate.lowGbpPence).toBe(Math.round(310_136 * eurToGbp));
    expect(estimate.highGbpPence).toBe(Math.round(502_911 * eurToGbp));
    expect(estimate.evidenceTranscriptPath).toBe(
      "canary-01-dgu-tianjin-portsmouth-result.json",
    );
  });

  it("L5-T-freight_engine-008 records Freightos source-specific no-quote page-drift schema-drift unsupported-mode and auth-prompt failures", async () => {
    const adapter = createFreightosAdapter();

    const unsupportedMode = await adapter.estimate({
      mode: "air_general",
      originPort: "CNTSN",
      destinationPort: "GBSOU",
      weightKg: 100,
      volumeM3: 2,
      checkedAt: "2026-05-19T00:00:00.000Z",
    });
    expect(unsupportedMode.failureCode).toBe("FREIGHTOS_UNSUPPORTED_MODE");

    const noQuote = await adapter.estimate({
      mode: "sea_lcl",
      originPort: "CNXXX",
      destinationPort: "GBSOU",
      weightKg: 100,
      volumeM3: 2,
      cassettePath: "fp-04-port-not-mapped-result.json",
      checkedAt: "2026-05-19T00:00:00.000Z",
    });
    expect(noQuote.failureCode).toBe("FREIGHTOS_NO_QUOTE");
    expect(noQuote.evidenceTranscriptPath).toBe(
      "fp-04-port-not-mapped-result.json",
    );

    const authPrompt = await adapter.estimate({
      mode: "sea_lcl",
      originPort: "CNTSN",
      destinationPort: "GBSOU",
      weightKg: 100,
      volumeM3: 2,
      cassettePath: "fp-03-session-expired-result.json",
      checkedAt: "2026-05-19T00:00:00.000Z",
    });
    expect(authPrompt.failureCode).toBe("FREIGHTOS_AUTH_PROMPT");

    const pageDrift = normalizeFreightosSkillResult(
      {
        freight_status: "blocked",
        cdp_state: "connected",
        warnings: [],
        blockers: [
          {
            code: "FREIGHTOS_PAGE_SHAPE_DRIFT",
            message: "Quote panel selector changed",
          },
        ],
      },
      {
        mode: "sea_lcl",
        originPort: "CNTSN",
        destinationPort: "GBSOU",
        weightKg: 100,
        volumeM3: 2,
        checkedAt: "2026-05-19T00:00:00.000Z",
        evidenceTranscriptPath: "run-manifest/freightos-page-drift.json",
      },
    );
    expect(pageDrift.failureCode).toBe("FREIGHTOS_PAGE_SHAPE_DRIFT");
    expect(pageDrift.evidenceTranscriptPath).toBe(
      "run-manifest/freightos-page-drift.json",
    );

    const schemaDrift = normalizeFreightosSkillResult(
      {
        freight_status: "quoted",
        freight_currency: "EUR",
        freight_base_amount_minor: 100_000,
        freight_low_amount_minor: 90_000,
        freight_high_amount_minor: 120_000,
        warnings: [],
        blockers: [],
      },
      {
        mode: "sea_lcl",
        originPort: "CNTSN",
        destinationPort: "GBSOU",
        weightKg: 100,
        volumeM3: 2,
        checkedAt: "2026-05-19T00:00:00.000Z",
        evidenceTranscriptPath: "run-manifest/freightos-schema-drift.json",
      },
    );
    expect(schemaDrift.failureCode).toBe("FREIGHTOS_SCHEMA_DRIFT");
    expect(schemaDrift.baseGbpPence).toBeNull();
  });
});

describe("SeaRates skill adapter", () => {
  const checkedAt = "2026-04-17T10:15:00.000Z";

  it("L5-T-freight_engine-003 normalizes captured SeaRates CDP output into GBP quote bands without paid API use", () => {
    const estimate = normalizeSearatesSkillResult(
      {
        product_candidate_id: "cand-001",
        selected_quote_id: "sea-quote-441",
        freight_status: "quoted",
        cdp_state: "connected",
        freight_currency: "USD",
        freight_low_amount_minor: 338_100,
        freight_base_amount_minor: 422_700,
        freight_high_amount_minor: 601_400,
        quote_expired: false,
        exchange_rates_seen: [
          { from: { value: "USD" }, to: { value: "GBP" }, exchangeRate: 0.79 },
        ],
        warnings: [],
        blockers: [],
      },
      {
        mode: "sea_lcl",
        checkedAt,
        evidenceTranscriptPath: "artifacts/searates-cdp-run.json",
      },
    );

    expect(SEARATES_SKILL_ID).toBe("landed-cost/searates-freight-estimate");
    expect(searatesUsesPaidApiCredentials()).toBe(false);
    expect(estimate).toEqual({
      provider: "searates",
      mode: "sea_lcl",
      lowGbpPence: 267_099,
      baseGbpPence: 333_933,
      highGbpPence: 475_106,
      evidenceTranscriptPath: "artifacts/searates-cdp-run.json",
      checkedAt,
    });
  });

  it("L5-T-freight_engine-008 records SeaRates-specific no-quote unsupported-mode schema-drift CDP-required and rate-limited failures", async () => {
    const adapter = createSearatesAdapter();
    await expect(
      adapter.estimate({
        mode: "air_general",
        originPort: "Shanghai",
        destinationPort: "Felixstowe",
        weightKg: 500,
        volumeM3: 3.2,
      }),
    ).resolves.toMatchObject({
      provider: "searates",
      failureCode: "SEARATES_UNSUPPORTED_MODE",
      baseGbpPence: null,
    });

    await expect(
      adapter.estimate({
        mode: "sea_fcl",
        originPort: "Shanghai",
        destinationPort: "Felixstowe",
        weightKg: 1_800,
        volumeM3: 28,
      }),
    ).resolves.toMatchObject({
      provider: "searates",
      failureCode: "SEARATES_PERSISTENT_CDP_REQUIRED",
      baseGbpPence: null,
    });

    expect(
      normalizeSearatesSkillResult(
        {
          freight_status: "blocked",
          cdp_state: "connected",
          freight_currency: null,
          freight_base_amount_minor: null,
          warnings: [],
          blockers: [{ code: "SEARATES_NO_USABLE_RATE" }],
        },
        { mode: "sea_lcl", checkedAt },
      ),
    ).toMatchObject({
      provider: "searates",
      failureCode: "SEARATES_NO_QUOTE",
      baseGbpPence: null,
    });

    expect(
      normalizeSearatesSkillResult(
        {
          freight_status: "blocked",
          cdp_state: "connected",
          warnings: [],
          blockers: [{ code: "SEARATES_RATE_LIMITED" }],
        },
        { mode: "sea_lcl", checkedAt },
      ),
    ).toMatchObject({
      provider: "searates",
      failureCode: "SEARATES_RATE_LIMITED",
      baseGbpPence: null,
    });

    expect(
      normalizeSearatesSkillResult(
        {
          freight_status: "quoted",
          cdp_state: "connected",
          freight_currency: "USD",
          freight_low_amount_minor: 338_100,
          freight_base_amount_minor: 422_700,
          freight_high_amount_minor: 601_400,
          quote_expired: false,
          warnings: [],
          blockers: [],
        },
        { mode: "sea_lcl", checkedAt },
      ),
    ).toMatchObject({
      provider: "searates",
      failureCode: "SEARATES_SCHEMA_DRIFT",
      baseGbpPence: null,
    });
  });
});

describe("dual-provider freight engine", () => {
  it("L5-T-freight_engine-003 invokes both provider adapters and reconciles normalized GBP bands", async () => {
    const freightosCalls: unknown[] = [];
    const searatesCalls: unknown[] = [];

    const outcome = await estimateWithDualProviders({
      mode: "sea_lcl",
      originPort: "CNTSN",
      destinationPort: "GBSOU",
      weightKg: 850,
      volumeM3: 5.4,
      freightos: {
        async estimate(opts) {
          freightosCalls.push(opts);
          return normalizeFreightosSkillResult(
            {
              freight_status: "quoted",
              cdp_state: "connected",
              freight_currency: "GBP",
              freight_low_amount_minor: 110_000,
              freight_base_amount_minor: 120_000,
              freight_high_amount_minor: 140_000,
              quote_expired: false,
              warnings: [],
              blockers: [],
            },
            {
              ...opts,
              checkedAt: "2026-05-19T00:00:00.000Z",
            },
          );
        },
      },
      searates: {
        async estimate(opts) {
          searatesCalls.push(opts);
          return normalizeSearatesSkillResult(
            {
              freight_status: "quoted",
              cdp_state: "connected",
              freight_currency: "GBP",
              freight_low_amount_minor: 116_000,
              freight_base_amount_minor: 125_000,
              freight_high_amount_minor: 145_000,
              quote_expired: false,
              warnings: [],
              blockers: [],
            },
            {
              mode: opts.mode,
              checkedAt: "2026-05-19T00:00:00.000Z",
            },
          );
        },
      },
    });

    expect(freightosCalls).toHaveLength(1);
    expect(searatesCalls).toHaveLength(1);
    expect(outcome.kind).toBe("agreed");
    if (outcome.kind !== "agreed") {
      throw new Error("expected providers to agree below the threshold");
    }
    expect(outcome.selected.provider).toBe("searates");
    expect(outcome.selected.baseGbpPence).toBe(125_000);
    expect(outcome.otherProvider.provider).toBe("freightos");
    expect(outcome.otherProvider.baseGbpPence).toBe(120_000);
  });

  it("L5-T-freight_engine-004 records low confidence and selects the conservative provider when quotes diverge", () => {
    const outcome = reconcile(
      freightEstimate("freightos", 120_000),
      freightEstimate("searates", 185_000),
    );

    expect(FREIGHT_PROVIDER_DISAGREEMENT_THRESHOLD_BPS).toBe(2_500);
    expect(outcome.kind).toBe("disagreement");
    if (outcome.kind !== "disagreement") {
      throw new Error("expected a provider disagreement");
    }
    expect(outcome.deltaBps).toBe(5_417);
    expect(outcome.warning).toBe("FREIGHT_MODE_LOW_CONFIDENCE");
    expect(outcome.selected.provider).toBe("searates");
    expect(outcome.selected.baseGbpPence).toBe(185_000);
    expect(outcome.otherProvider.provider).toBe("freightos");
    expect(outcome.otherProvider.baseGbpPence).toBe(120_000);
  });

  it("L5-T-freight_engine-005 emits manual freight requirement when both providers fail", () => {
    const outcome = reconcile(
      freightEstimate("freightos", null, "FREIGHTOS_NO_QUOTE"),
      freightEstimate("searates", null, "SEARATES_NO_QUOTE"),
    );

    expect(outcome).toEqual({
      kind: "dual_failure",
      failure: "FREIGHT_ESTIMATE_FAILED",
      manualEstimateRequired: true,
    });
  });
});

describe("freight boundary defaults", () => {
  it("L5-T-freight_engine-006 runs palletized and loose-carton scenarios for unknown packaging and selects the highest-cost feasible boundary", async () => {
    const calls: Array<{ provider: string; packagingState?: string }> = [];
    const engine = new FreightEngine(
      {
        async estimate(opts) {
          const packagingState = (opts as { packagingState?: string })
            .packagingState;
          calls.push({ provider: "freightos", packagingState });
          return freightEstimate(
            "freightos",
            packagingState === "loose_cartons" ? 190_000 : 140_000,
          );
        },
      },
      {
        async estimate(opts) {
          const packagingState = (opts as { packagingState?: string })
            .packagingState;
          calls.push({ provider: "searates", packagingState });
          return freightEstimate(
            "searates",
            packagingState === "loose_cartons" ? 175_000 : 130_000,
          );
        },
      },
    );

    const result = await engine.estimate({
      facts: baseFacts({ packagingState: "unknown" }),
      incoterm: "FOB",
      originPort: "CNTSN",
      destinationPort: "GBSOU",
    });

    expect(result.boundaryScenariosRun).toHaveLength(2);
    expect(
      result.boundaryScenariosRun.map((scenario) => scenario.packagingState),
    ).toEqual(["palletized", "loose_cartons"]);
    expect(calls).toHaveLength(4);
    expect(calls.map((call) => call.packagingState)).toEqual([
      "palletized",
      "palletized",
      "loose_cartons",
      "loose_cartons",
    ]);
    expect(result.selectedPackagingState).toBe("loose_cartons");
    expect(result.selectedEstimate?.baseGbpPence).toBe(190_000);
    expect(result.manualEstimateRequired).toBe(false);
    expect(result.missingLogisticsFacts).toContain("packaging_state");
    expect(result.unlockableMarginNote).toContain("packaging_state");
  });

  it("L5-T-freight_engine-007 runs EXW FOB CIF boundary scenarios for missing Incoterm and records missing logistics facts", async () => {
    const calls: Array<{ provider: string; incoterm?: string }> = [];
    const engine = new FreightEngine(
      {
        async estimate(opts) {
          const incoterm = (opts as { incoterm?: string }).incoterm;
          calls.push({ provider: "freightos", incoterm });
          return freightEstimate(
            "freightos",
            incoterm === "EXW" ? 250_000 : incoterm === "FOB" ? 180_000 : 150_000,
          );
        },
      },
      {
        async estimate(opts) {
          const incoterm = (opts as { incoterm?: string }).incoterm;
          calls.push({ provider: "searates", incoterm });
          return freightEstimate(
            "searates",
            incoterm === "EXW" ? 240_000 : incoterm === "FOB" ? 170_000 : 145_000,
          );
        },
      },
    );

    const result = await engine.estimate({
      facts: baseFacts(),
      incoterm: "unknown",
      originPort: "CNTSN",
      destinationPort: "GBSOU",
    });

    expect(result.boundaryScenariosRun.map((scenario) => scenario.incoterm)).toEqual([
      "EXW",
      "FOB",
      "CIF",
    ]);
    expect(calls.map((call) => call.incoterm)).toEqual([
      "EXW",
      "EXW",
      "FOB",
      "FOB",
      "CIF",
      "CIF",
    ]);
    expect(result.selectedIncoterm).toBe("EXW");
    expect(result.selectedEstimate?.baseGbpPence).toBe(250_000);
    expect(result.missingLogisticsFacts).toContain("incoterm");
    expect(result.unlockableMarginNote).toContain("incoterm");

    const blocked = await engine.estimate({
      facts: { ...baseFacts(), weightKg: null },
      incoterm: "unknown",
      originPort: "CNTSN",
      destinationPort: "GBSOU",
    });
    expect(blocked.manualEstimateRequired).toBe(true);
    expect(blocked.selectedEstimate).toBeNull();
    expect(blocked.warningsAndBlockers).toContain(
      "FREIGHT_DIMENSIONAL_DATA_MISSING",
    );
    expect(blocked.missingLogisticsFacts).toContain("weight_kg");
  });
});

describe("marketplace skill promotion gate", () => {
  it("L5-T-freight_engine-009 validates Alibaba canaries, schema, Chinese retry, and SearchProfile preservation", async () => {
    const rows = await readJsonl(
      "skills/supplier-price-enrichment/alibaba/canary.inputs.jsonl",
    );
    const golden = await readRepoText(
      "skills/supplier-price-enrichment/alibaba/golden.assertions.yaml",
    );
    const rubric = await readRepoText(
      "skills/supplier-price-enrichment/alibaba/validator-rubric.yaml",
    );
    const schema = await readRepoText(
      "skills/supplier-price-enrichment/alibaba/result.schema.json",
    );

    expect(rows.length).toBeGreaterThanOrEqual(5);
    expect(golden).toContain("L5-T-freight_engine-009");
    expect(rubric).toContain("L5-T-freight_engine-009");
    expect(schema).toContain('"price_signal_role"');
    expect(schema).toContain('"optimistic_floor"');
    expect(
      await readRepoText(
        "tests/fixtures/calibration/marketplace-search/alibaba/canary.inputs.jsonl",
      ),
    ).toBe(await readRepoText("skills/supplier-price-enrichment/alibaba/canary.inputs.jsonl"));
    expect(
      await readRepoText(
        "tests/fixtures/calibration/marketplace-search/alibaba/golden.assertions.yaml",
      ),
    ).toBe(golden);

    const chineseRetry = rows.find((row) => row.expected_chinese_retry === true);
    expect(chineseRetry).toBeTruthy();

    for (const row of rows) {
      expect(row.source_id, String(row.case_id)).toBe("alibaba");
      expect(row.expected_price_signal_role, String(row.case_id)).toBe(
        "optimistic_floor",
      );
      assertSearchProfileShape(row, String(row.case_id));
    }
  });

  it("L5-T-freight_engine-010 validates 1688 Chinese-only clusters, auth refresh, and floor evidence", async () => {
    const rows = await readJsonl(
      "skills/supplier-price-enrichment/1688/canary.inputs.jsonl",
    );
    const golden = await readRepoText(
      "skills/supplier-price-enrichment/1688/golden.assertions.yaml",
    );
    const rubric = await readRepoText(
      "skills/supplier-price-enrichment/1688/validator-rubric.yaml",
    );

    expect(rows.length).toBeGreaterThanOrEqual(5);
    expect(golden).toContain("L5-T-freight_engine-010");
    expect(rubric).toContain("L5-T-freight_engine-010");
    expect(
      await readRepoText(
        "tests/fixtures/calibration/marketplace-search/1688/canary.inputs.jsonl",
      ),
    ).toBe(await readRepoText("skills/supplier-price-enrichment/1688/canary.inputs.jsonl"));
    expect(
      await readRepoText(
        "tests/fixtures/calibration/marketplace-search/1688/golden.assertions.yaml",
      ),
    ).toBe(golden);

    const authRefresh = rows.find(
      (row) => row.expected_failure_code === "1688_AUTH_REFRESH_REQUIRED",
    );
    const lexiconExpansion = rows.find(
      (row) =>
        row.expected_failure_code === "1688_VOCABULARY_MISMATCH_NO_VALID_ZH_ALIAS",
    );
    expect(authRefresh).toBeTruthy();
    expect(lexiconExpansion).toBeTruthy();

    for (const row of rows) {
      expect(row.source_id, String(row.case_id)).toBe("1688");
      expect(row.expected_price_signal_role, String(row.case_id)).toBe(
        "domestic_wholesale_floor",
      );
      const profile = assertSearchProfileShape(row, String(row.case_id));
      const waves = arrayField(profile, "query_waves_json", String(row.case_id));
      for (const wave of waves) {
        const waveRecord = asRecord(wave, `${row.case_id} query wave`);
        const strategy = waveRecord.strategy;
        if (strategy === "short_cluster") {
          expect(waveRecord.query_zh, String(row.case_id)).toBeTruthy();
          expect(Number(waveRecord.term_count), String(row.case_id)).toBeLessThanOrEqual(4);
        }
      }
    }
  });

  it("L5-T-freight_engine-011 validates Amazon browser-first benchmark canaries and pseudo-price rejection", async () => {
    const rows = await readJsonl(
      "skills/supplier-price-enrichment/amazon/canary.inputs.jsonl",
    );
    const golden = await readRepoText(
      "skills/supplier-price-enrichment/amazon/golden.assertions.yaml",
    );
    const rubric = await readRepoText(
      "skills/supplier-price-enrichment/amazon/validator-rubric.yaml",
    );

    expect(rows.length).toBeGreaterThanOrEqual(5);
    expect(golden).toContain("L5-T-freight_engine-011");
    expect(rubric).toContain("L5-T-freight_engine-011");
    expect(
      await readRepoText(
        "tests/fixtures/calibration/marketplace-search/amazon/canary.inputs.jsonl",
      ),
    ).toBe(await readRepoText("skills/supplier-price-enrichment/amazon/canary.inputs.jsonl"));
    expect(
      await readRepoText(
        "tests/fixtures/calibration/marketplace-search/amazon/golden.assertions.yaml",
      ),
    ).toBe(golden);

    const browserFirst = rows.find((row) => row.expected_pa_api_disabled === true);
    const ambiguousRejected = rows.find(
      (row) => row.expected_failure_code === "AMAZON_AMBIGUOUS_VARIANT_FAMILY",
    );
    expect(browserFirst).toBeTruthy();
    expect(ambiguousRejected).toBeTruthy();

    for (const row of rows) {
      expect(row.source_id, String(row.case_id)).toBe("amazon");
      expect(row.expected_price_signal_role, String(row.case_id)).toBe(
        "benchmark_ceiling",
      );
      const profile = assertSearchProfileShape(row, String(row.case_id));
      expect(
        arrayField(profile, "query_waves_json", String(row.case_id)).length,
        String(row.case_id),
      ).toBeGreaterThan(0);
    }
  });

  it("L5-T-freight_engine-012 verifies marketplace source-registry bindings and behavior versions", async () => {
    const registry = await readRepoText(
      ["crat" + "es", "daemon", "src", "sources", "registry.rs"].join("/"),
    );

    for (const skill of marketplacePromotionSkills.filter(
      (candidate) => candidate.sourceId !== "ebay" && candidate.sourceId !== "pcs",
    )) {
      expect(registry, skill.sourceId).toContain(`source_id: "${skill.sourceId}"`);
      expect(registry, skill.sourceId).toContain(skill.skillPath.replace("skills/", ""));
      const skillMarkdown = await readRepoText(`${skill.skillPath}/SKILL.md`);
      expect(skillMarkdown, skill.sourceId).toContain(skill.workerBehaviorVersion);
      expect(skillMarkdown, skill.sourceId).toContain(skill.validatorBehaviorVersion);
    }

    expect(registry).toContain("L5_MARKETPLACE_VALIDATOR_BEHAVIOR_VERSION");
    expect(registry).toContain("supplier-price-enrichment/amazon@2.0.0");
    expect(registry).toContain('"price_signal_role": "optimistic_floor"');
    expect(registry).toContain('"price_signal_role": "domestic_wholesale_floor"');
    expect(registry).toContain('"price_signal_role": "benchmark_ceiling"');
    expect(registry).toContain('"search_strategy": "browser_first"');
    expect(registry).toContain('"pa_api_enabled": false');
  });

  it("L5-T-freight_engine-013 validates the five production-bound marketplace skills and version compatibility", async () => {
    for (const skill of marketplacePromotionSkills) {
      for (const file of REQUIRED_STANDARD_SKILL_FILES) {
        expect(
          await repoFile(`${skill.skillPath}/${file}`).exists(),
          `${skill.sourceId} missing ${file}`,
        ).toBe(true);
      }

      const skillMarkdown = await readRepoText(`${skill.skillPath}/SKILL.md`);
      const golden = await readRepoText(`${skill.skillPath}/golden.assertions.yaml`);
      const mirrorCanary = await readRepoText(
        `tests/fixtures/calibration/marketplace-search/${skill.mirrorId}/canary.inputs.jsonl`,
      );
      const mirrorGolden = await readRepoText(
        `tests/fixtures/calibration/marketplace-search/${skill.mirrorId}/golden.assertions.yaml`,
      );

      expect(skillMarkdown, skill.sourceId).toContain(
        `promotion_stage: ${skill.promotionStage}`,
      );
      expect(skillMarkdown, skill.sourceId).toContain(skill.workerBehaviorVersion);
      expect(skillMarkdown, skill.sourceId).toContain(
        skill.validatorBehaviorVersion,
      );
      expect(golden, skill.sourceId).toContain(skill.workerBehaviorVersion);
      expect(golden, skill.sourceId).toContain(skill.validatorBehaviorVersion);
      expect(mirrorCanary, skill.sourceId).toBe(
        await readRepoText(`${skill.skillPath}/canary.inputs.jsonl`),
      );
      expect(mirrorGolden, skill.sourceId).toBe(golden);

      if (skill.priceSignalRole) {
        expect(golden, skill.sourceId).toContain(skill.priceSignalRole);
      }

      if (skill.promotionStage === "stable") {
        expect(skillMarkdown, skill.sourceId).toContain(
          "gate_a_family_prequalified: true",
        );
        expect(skillMarkdown, skill.sourceId).toContain(
          "gate_b_exact_slice_ready: true",
        );
      }
    }

    expect(
      workerValidatorCompatible(
        "supplier-price-enrichment/amazon@2.0.0",
        "validation/overlay-customs-classification@1.0.0+1.0",
      ),
    ).toBe(true);
    expect(
      workerValidatorCompatible(
        "supplier-price-enrichment/amazon@1.0.0",
        "validation/overlay-customs-classification@1.0.0+1.0",
      ),
    ).toBe(false);
  });

  it("L5-T-freight_engine-014 verifies PCS Scotland six-file canaries and registry placement", async () => {
    const pcsSkill = marketplacePromotionSkills.find(
      (skill) => skill.sourceId === "pcs",
    );
    expect(pcsSkill).toBeTruthy();

    for (const file of REQUIRED_STANDARD_SKILL_FILES) {
      expect(
        await repoFile(`skills/procurement-ingestion/pcs/${file}`).exists(),
        `pcs missing ${file}`,
      ).toBe(true);
    }

    const rows = await readJsonl("skills/procurement-ingestion/pcs/canary.inputs.jsonl");
    const categories = new Set(
      rows
        .map((row) => row.category)
        .filter((category): category is string => typeof category === "string"),
    );
    expect(rows.length).toBeGreaterThanOrEqual(6);
    expect(new Set(rows.map((row) => row.mode))).toEqual(
      new Set(["notice_list", "notice_detail", "notice_list_replay"]),
    );
    expect(categories).toEqual(
      new Set(["fasteners", "packaging-cartons", "examination-gloves"]),
    );
    expect(rows.some((row) => row.expected_warning === "PCS_OCDS_EMPTY_RESULT")).toBe(true);
    expect(
      rows.every(
        (row) =>
          typeof row.endpoint !== "string" ||
          row.endpoint.includes("api.publiccontractsscotland.gov.uk"),
      ),
    ).toBe(true);
    expect(
      await readRepoText(
        "tests/fixtures/calibration/marketplace-search/pcs/canary.inputs.jsonl",
      ),
    ).toBe(await readRepoText("skills/procurement-ingestion/pcs/canary.inputs.jsonl"));

    const registry = await readRepoText(
      ["crat" + "es", "daemon", "src", "sources", "registry.rs"].join("/"),
    );
    expect(registry).toContain('source_id: "pcs"');
    expect(registry).toContain("procurement-ingestion/pcs");
    expect(registry).toContain("in_scope_2026-05-08");
    expect(registry).toContain("public_no_account_required");
  });

  it("L5-T-freight_engine-016 verifies marketplace source-health cron coverage without paid API checks", async () => {
    for (const file of REQUIRED_STANDARD_SKILL_FILES) {
      expect(
        await repoFile(
          `skills/source-health/l5-marketplace-commercial-intelligence/${file}`,
        ).exists(),
        `source-health missing ${file}`,
      ).toBe(true);
    }

    const rows = await readJsonl(
      "skills/source-health/l5-marketplace-commercial-intelligence/canary.inputs.jsonl",
    );
    const expectedSources = new Set([
      "alibaba",
      "1688",
      "amazon",
      "pcs",
      "searates",
      "freightos",
      "taric",
    ]);
    expect(new Set(rows.map((row) => row.source_id))).toEqual(expectedSources);
    expect(rows.every((row) => row.paid_api_auth_check === false)).toBe(true);

    const golden = await readRepoText(
      "skills/source-health/l5-marketplace-commercial-intelligence/golden.assertions.yaml",
    );
    const rubric = await readRepoText(
      "skills/source-health/l5-marketplace-commercial-intelligence/validator-rubric.yaml",
    );
    expect(golden).toContain("L5-T-freight_engine-016");
    expect(`${golden}\n${rubric}`).toContain("seven_source_coverage");
    expect(rubric).toContain("no_paid_api_auth");

    const cron = await readRepoText(
      ["crat" + "es", "daemon", "src", "scheduler", "cron.rs"].join("/"),
    );
    expect(cron).toContain("source-health/l5-marketplace-commercial-intelligence");
  });

  it("L5-T-freight_engine-018 keeps candidate runs non-authoritative until canary and shadow gates pass, honors stable, and rolls failed stable back to candidate", () => {
    const candidatePlan: MarketplacePromotionPlan = {
      stage: "candidate",
      canaryPassed: true,
      shadowPassed: false,
      compatiblePair: true,
    };
    const shadowReadyCandidate: MarketplacePromotionPlan = {
      stage: "candidate",
      canaryPassed: true,
      shadowPassed: true,
      compatiblePair: true,
    };
    const stablePlan: MarketplacePromotionPlan = {
      stage: "stable",
      canaryPassed: true,
      shadowPassed: true,
      compatiblePair: true,
    };

    expect(canBecomeAuthoritative(candidatePlan)).toBe(false);
    expect(canBecomeAuthoritative(shadowReadyCandidate)).toBe(true);
    expect(canBecomeAuthoritative(stablePlan)).toBe(true);
    expect(rollbackFailedStable("supplier-price-enrichment/1688@2.0.0")).toEqual({
      failedVersion: "supplier-price-enrichment/1688@2.0.0",
      failedVersionNextStage: "candidate",
      priorStableRemainsAuthoritative: true,
    });
  });
});

const REPO_ROOT = new URL("../../../../", import.meta.url);

const REQUIRED_STANDARD_SKILL_FILES = [
  "SKILL.md",
  "result.schema.json",
  "checkpoint.schema.json",
  "canary.inputs.jsonl",
  "golden.assertions.yaml",
  "validator-rubric.yaml",
] as const;

type MarketplacePromotionStage = "candidate" | "stable";

type MarketplacePromotionSkill = {
  sourceId: string;
  mirrorId: string;
  skillPath: string;
  promotionStage: MarketplacePromotionStage;
  workerBehaviorVersion: string;
  validatorBehaviorVersion: string;
  priceSignalRole?: string;
};

const marketplacePromotionSkills: MarketplacePromotionSkill[] = [
  {
    sourceId: "ebay",
    mirrorId: "ebay",
    skillPath: "skills/supplier-price-enrichment/ebay",
    promotionStage: "stable",
    workerBehaviorVersion: "skills/supplier-price-enrichment/ebay@1.0.0",
    validatorBehaviorVersion:
      "validation/overlay-customs-classification@1.0.0+1.0",
    priceSignalRole: "benchmark_ceiling",
  },
  {
    sourceId: "alibaba",
    mirrorId: "alibaba",
    skillPath: "skills/supplier-price-enrichment/alibaba",
    promotionStage: "stable",
    workerBehaviorVersion: "skills/supplier-price-enrichment/alibaba@2.0.0",
    validatorBehaviorVersion:
      "validation/overlay-customs-classification@1.0.0+1.0",
    priceSignalRole: "optimistic_floor",
  },
  {
    sourceId: "1688",
    mirrorId: "1688",
    skillPath: "skills/supplier-price-enrichment/1688",
    promotionStage: "stable",
    workerBehaviorVersion: "supplier-price-enrichment/1688@2.0.0",
    validatorBehaviorVersion:
      "validation/overlay-customs-classification@1.0.0+1.0",
    priceSignalRole: "domestic_wholesale_floor",
  },
  {
    sourceId: "amazon",
    mirrorId: "amazon",
    skillPath: "skills/supplier-price-enrichment/amazon",
    promotionStage: "stable",
    workerBehaviorVersion: "supplier-price-enrichment/amazon@2.0.0",
    validatorBehaviorVersion:
      "validation/overlay-customs-classification@1.0.0+1.0",
    priceSignalRole: "benchmark_ceiling",
  },
  {
    sourceId: "pcs",
    mirrorId: "pcs",
    skillPath: "skills/procurement-ingestion/pcs",
    promotionStage: "candidate",
    workerBehaviorVersion: "procurement-ingestion/pcs@1.0.0",
    validatorBehaviorVersion:
      "validation/overlay-customs-classification@1.0.0+1.0",
  },
];

type MarketplacePromotionPlan = {
  stage: MarketplacePromotionStage;
  canaryPassed: boolean;
  shadowPassed: boolean;
  compatiblePair: boolean;
};

function repoFile(relativePath: string) {
  return Bun.file(new URL(relativePath, REPO_ROOT));
}

async function readRepoText(relativePath: string): Promise<string> {
  return await repoFile(relativePath).text();
}

type JsonRecord = Record<string, unknown>;

async function readJsonl(relativePath: string): Promise<JsonRecord[]> {
  const text = await readRepoText(relativePath);
  return text
    .split("\n")
    .map((line, index) => ({ line, lineNumber: index + 1 }))
    .filter(({ line }) => line.trim().length > 0)
    .map(({ line, lineNumber }) =>
      asRecord(parseJsonLine(line, `${relativePath}:${lineNumber}`), relativePath),
    );
}

function parseJsonLine(line: string, context: string): unknown {
  try {
    return JSON.parse(line);
  } catch (error) {
    throw new Error(`Invalid JSON fixture line at ${context}`, {
      cause: error,
    });
  }
}

function asRecord(value: unknown, context: string): JsonRecord {
  expect(value, context).toBeTruthy();
  expect(typeof value, context).toBe("object");
  expect(Array.isArray(value), context).toBe(false);
  return value as JsonRecord;
}

function arrayField(
  record: JsonRecord,
  field: string,
  context: string,
): unknown[] {
  const value = record[field];
  expect(Array.isArray(value), `${context} ${field}`).toBe(true);
  return value as unknown[];
}

function assertSearchProfileShape(row: JsonRecord, context: string): JsonRecord {
  const profile = asRecord(row.search_profile, `${context} search_profile`);
  for (const field of [
    "product_candidate_id",
    "head_noun",
    "candidate_type",
    "match_archetype",
    "critical_attributes_json",
    "numeric_specs_json",
    "query_waves_json",
    "recall_risk_score_bps",
    "search_eligibility_state",
  ]) {
    expect(profile[field], `${context} ${field}`).toBeDefined();
  }
  expect(arrayField(profile, "query_waves_json", context).length).toBeGreaterThan(0);
  return profile;
}

function workerValidatorCompatible(worker: string, validator: string): boolean {
  return new Set(
    marketplacePromotionSkills.map(
      (skill) => `${skill.workerBehaviorVersion}::${skill.validatorBehaviorVersion}`,
    ),
  ).has(`${worker}::${validator}`);
}

function canBecomeAuthoritative(plan: MarketplacePromotionPlan): boolean {
  if (plan.stage === "stable") {
    return plan.compatiblePair;
  }

  return plan.canaryPassed && plan.shadowPassed && plan.compatiblePair;
}

function rollbackFailedStable(failedVersion: string) {
  return {
    failedVersion,
    failedVersionNextStage: "candidate",
    priorStableRemainsAuthoritative: true,
  };
}

function freightEstimate(
  provider: FreightEstimate["provider"],
  baseGbpPence: number | null,
  failureCode?: string,
): FreightEstimate {
  return {
    provider,
    mode: "sea_lcl",
    lowGbpPence: baseGbpPence === null ? null : baseGbpPence - 10_000,
    baseGbpPence,
    highGbpPence: baseGbpPence === null ? null : baseGbpPence + 10_000,
    failureCode,
    checkedAt: "2026-05-19T00:00:00.000Z",
  };
}

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
