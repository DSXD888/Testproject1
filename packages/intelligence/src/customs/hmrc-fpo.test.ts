import { describe, expect, it } from "bun:test";
import { createHash } from "node:crypto";

import {
  FPO_ACCESS_UNAVAILABLE,
  MultiSourceRecall,
  RECALL_SOURCE_FAMILY_ORDER,
  availableRecallSource,
  createRecallCandidate,
  evaluateRecallFamilyCoverage,
  interpretFpoHttpResponse,
  parseHmrcSearchReferenceRecall,
  parseHmrcTariffKeywordSearchRecall,
  probeHmrcFpoEndpointDecision,
  reconcileHmrcWithTaric,
} from "./multi-source-recall";
import {
  RECALL_QUERY_SOURCE_FAMILIES,
  countRecallSourceFamilies,
  gateDescriptionQuality,
  generateQueryWaves,
  planRecallQueryWaves,
} from "./description-quality";

interface CassetteFixture {
  case_id: string;
  endpoint_family: string;
  request: {
    method: string;
    url: string;
    headers: Record<string, string>;
    body_sha256: string | null;
    authenticated: boolean;
  };
  request_fingerprint: string;
  response: {
    status: number;
    headers: Record<string, string>;
    body_path: string;
    body_sha256: string;
  };
  recorded_at: string;
  recorder: string;
  expected_failure_mode: string | null;
}

describe("HMRC FPO endpoint discovery and public search-reference recall", () => {
  it("L5-T-hmrc_fpo-001 parses HMRC tariff keyword search into HS candidates", async () => {
    const cassette = await readCassette("tariff-keyword-search-fasteners.json");
    const bodyText = await readCassetteBody(cassette);

    const result = parseHmrcTariffKeywordSearchRecall(bodyText, {
      searchTerm: "stainless steel fasteners bolts",
      sourceUrl: cassette.request.url,
    });

    expect(cassette.endpoint_family).toBe("hmrc_tariff_keyword_search");
    expect(cassette.request.authenticated).toBe(false);
    expect(result).toMatchObject({
      sourceFamily: "hmrc_tariff_keyword_search",
      state: "agreed",
      failureMode: "none",
    });
    expect(result.candidates[0]).toMatchObject({
      code: "7318159590",
      commodityCode: "7318159590",
      level: "cn8",
      sourceFamily: "hmrc_tariff_keyword_search",
      legalEvidence: false,
      authorityRole: "retrieval_relevance_only",
    });
    expect(result.candidates[0]?.retrievalScoreBps).toBeGreaterThanOrEqual(5000);
  });

  it("L5-T-hmrc_fpo-002 parses the selected public replacement into retrieval-only candidates", async () => {
    const cassette = await readCassette("fpo-search-reference-medical-gloves.json");
    const bodyText = await readCassetteBody(cassette);

    const result = parseHmrcSearchReferenceRecall(bodyText, {
      searchTerm: "medical nitrile surgical gloves",
      sourceUrl: cassette.request.url,
    });

    expect(cassette.endpoint_family).toBe("hmrc_public_search_references");
    expect(cassette.request.authenticated).toBe(false);
    expect(await bodySha256(cassette.response.body_path)).toBe(
      cassette.response.body_sha256,
    );
    expect(result.status).toBe("available");
    expect(result.sourceFamily).toBe("hmrc_fpo_or_search_suggestion");
    expect(result.legalEvidence).toBe(false);
    expect(result.authorityRole).toBe("retrieval_relevance_only");
    expect(result.candidates[0]).toMatchObject({
      commodityCode: "4015120000",
      title: "Surgical gloves, of vulcanised rubber (PPE)",
      legalEvidence: false,
      authorityRole: "retrieval_relevance_only",
    });
  });

  it("L5-T-hmrc_fpo-003 probes HMRC endpoint decisions without creating procurement action", async () => {
    const decision = await probeHmrcFpoEndpointDecision({
      checkedAt: "2026-05-19T21:56:17Z",
    });

    expect(decision.fpoEndpoint.failureCode).toBe(FPO_ACCESS_UNAVAILABLE);
    expect(decision.fpoEndpoint.procurementActionCreated).toBe(false);
    expect(decision.publicReplacement.role).toBe("retrieval_relevance_only");
    expect(["available", "unavailable"]).toContain(decision.publicReplacement.status);
    expect([
      "restricted_requires_invitation_or_api_key",
      "network_unavailable",
    ]).toContain(decision.fpoEndpoint.status);
  });

  it("L5-T-hmrc_fpo-007 preserves HMRC authority when TARIC disagrees on UK national measures", () => {
    const reconciled = reconcileHmrcWithTaric({
      hmrcCommodityCode: "4015120000",
      taricCommodityCode: "4015190000",
      affectedMeasureKind: "UK national VAT additional code DE 6/17",
      taricCandidateTitle:
        "gloves, mittens and mitts, of vulcanised rubber excluding surgical gloves",
      sourceUrl: "skills/landed-cost/taric-cross-check/result.json",
    });

    expect(reconciled.authoritativeCommodityCode).toBe("4015120000");
    expect(reconciled.advisoryCandidates[0]).toMatchObject({
      commodityCode: "4015190000",
      legalEvidence: false,
      authorityRole: "advisory_cross_check",
    });
    expect(reconciled.divergences).toEqual([
      {
        code: "TARIC_ADVISORY_DIVERGENCE",
        hmrcAuthorityPreserved: true,
        taricRole: "advisory_cross_check",
        affectedMeasureKind: "UK national VAT additional code DE 6/17",
        reason:
          "TARIC disagreement is retained as evidence divergence only; HMRC remains authoritative for UK national measures.",
      },
    ]);
  });

  it("L5-T-hmrc_fpo-008 emits the existing FPO_ACCESS_UNAVAILABLE code for malformed bodies and continues", async () => {
    const cassette = await readCassette("fpo-search-reference-malformed.json");
    const bodyText = await readCassetteBody(cassette);

    const result = parseHmrcSearchReferenceRecall(bodyText, {
      searchTerm: "medical nitrile surgical gloves",
      sourceUrl: cassette.request.url,
    });

    expect(cassette.expected_failure_mode).toBe(FPO_ACCESS_UNAVAILABLE);
    expect(result.status).toBe("malformed");
    expect(result.failureCode).toBe(FPO_ACCESS_UNAVAILABLE);
    expect(result.continuationAllowed).toBe(true);
    expect(JSON.stringify(result)).not.toContain(["FPO", "UNAVAILABLE"].join("_"));
  });

  it("L5-T-hmrc_fpo-011 marks restricted FPO access stale and never guesses from an unauthenticated body", async () => {
    const cassette = await readCassette("fpo-restricted-403.json");
    const bodyText = await readCassetteBody(cassette);

    const result = interpretFpoHttpResponse({
      status: cassette.response.status,
      bodyText,
    });

    expect(cassette.request.authenticated).toBe(false);
    expect(result.status).toBe("restricted");
    expect(result.failureCode).toBe(FPO_ACCESS_UNAVAILABLE);
    expect(result.staleClassification).toBe(true);
    expect(result.guessingAllowed).toBe(false);
    expect(result.candidates).toEqual([]);
  });

  it("L5-T-hmrc_fpo-004 records all five agreeing recall source families", async () => {
    const attributes = acceptedAttributes("sterile nitrile examination gloves bulk pack");
    const recall = new MultiSourceRecall(
      Object.fromEntries(
        RECALL_SOURCE_FAMILY_ORDER.map((sourceFamily, index) => [
          sourceFamily,
          () => [
            createRecallCandidate({
              code: "4015120000",
              title: `${sourceFamily} surgical nitrile gloves`,
              sourceFamily,
              retrievalScoreBps: 8_500 - index * 100,
            }),
          ],
        ]),
      ),
    );

    const result = await recall.run(attributes, "33141420");

    expect(result.perSourceResults).toHaveLength(5);
    expect(result.candidateRecallAgreement[0]?.candidateCode).toBe("4015120000");
    expect(result.candidateRecallAgreement[0]?.agreement.map((entry) => entry.sourceFamily)).toEqual(
      RECALL_SOURCE_FAMILY_ORDER,
    );
    expect(result.candidateRecallAgreement[0]?.agreement.map((entry) => entry.state)).toEqual([
      "agreed",
      "agreed",
      "agreed",
      "agreed",
      "agreed",
    ]);
    expect(result.independentSourceFamiliesAvailable).toBe(5);
    expect(result.independentSourceFamiliesAgreeing).toBe(5);
    expect(result.classificationBlockers).toEqual([]);
    expect(result.autoClassificationAllowed).toBe(true);
  });

  it("L5-T-hmrc_fpo-005 records family divergence and emits CLASSIFICATION_AMBIGUOUS", async () => {
    const attributes = acceptedAttributes("sterile nitrile examination gloves bulk pack");
    const recall = new MultiSourceRecall({
      hmrc_fpo_or_search_suggestion: () => [
        recallCandidate("4015120000", "surgical gloves", "hmrc_fpo_or_search_suggestion"),
      ],
      cpv_chapter_prior: () => [
        recallCandidate("4015190000", "protective rubber gloves", "cpv_chapter_prior"),
      ],
      rule_based_chapter_hint: () => [
        recallCandidate("4015120000", "medical surgical gloves", "rule_based_chapter_hint"),
      ],
      hmrc_tariff_keyword_search: () => [
        recallCandidate("3926200000", "plastic protective gloves", "hmrc_tariff_keyword_search"),
      ],
      atr_codebook_reuse: () => [
        recallCandidate("4015190000", "non-surgical rubber gloves", "atr_codebook_reuse"),
      ],
    });

    const result = await recall.run(attributes);
    const firstAgreement = result.candidateRecallAgreement[0]?.agreement ?? [];

    expect(result.independentSourceFamiliesAvailable).toBe(5);
    expect(firstAgreement.map((entry) => entry.sourceFamily)).toEqual(
      RECALL_SOURCE_FAMILY_ORDER,
    );
    expect(firstAgreement.map((entry) => entry.state)).toContain("diverged");
    expect(result.classificationBlockers).toEqual(["CLASSIFICATION_AMBIGUOUS"]);
    expect(result.autoClassificationAllowed).toBe(false);
  });

  it("L5-T-hmrc_fpo-006 keeps FPO-only recall non-auto and retrieval-only", async () => {
    const attributes = acceptedAttributes("sterile nitrile examination gloves bulk pack");
    const recall = new MultiSourceRecall({
      hmrc_fpo_or_search_suggestion: () => [
        recallCandidate(
          "4015120000",
          "retrieval-only surgical glove suggestion",
          "hmrc_fpo_or_search_suggestion",
        ),
      ],
    });

    const result = await recall.run(attributes);

    expect(result.independentSourceFamiliesAvailable).toBe(1);
    expect(result.independentSourceFamiliesAgreeing).toBe(1);
    expect(result.autoClassificationAllowed).toBe(false);
    expect(result.perSourceResults.map((entry) => entry.state)).toEqual([
      "agreed",
      "unavailable",
      "unavailable",
      "unavailable",
      "unavailable",
    ]);
    expect(result.perSourceResults[0]?.candidates[0]).toMatchObject({
      legalEvidence: false,
      authorityRole: "retrieval_relevance_only",
    });
  });

  it("L5-T-hmrc_fpo-012 accepts four non-FPO families when HS Ping is declined and FPO is unavailable", () => {
    const evaluation = evaluateRecallFamilyCoverage({
      sourceResults: [
        {
          ...availableRecallSource("hmrc_fpo_or_search_suggestion"),
          status: "unavailable",
          failureCode: FPO_ACCESS_UNAVAILABLE,
        },
        availableRecallSource("hmrc_tariff_keyword_search"),
        availableRecallSource("cpv_chapter_prior"),
        availableRecallSource("rule_based_chapter_hint"),
        availableRecallSource("atr_codebook_reuse"),
      ],
      declinedSources: ["hs_ping"],
    });

    expect(evaluation.green).toBe(true);
    expect(evaluation.declinedSources).toEqual(["hs_ping"]);
    expect(evaluation.activeSources).not.toContain("hmrc_fpo_or_search_suggestion");
    expect(evaluation.activeSources).toEqual([
      "hmrc_tariff_keyword_search",
      "cpv_chapter_prior",
      "rule_based_chapter_hint",
      "atr_codebook_reuse",
    ]);
    expect(evaluation.notes[0]).toContain("FPO unavailable is tolerated");
  });

  it("L5-T-hmrc_fpo-009 rejects generic service and unsafe bundle descriptions before recall", () => {
    const generic = gateDescriptionQuality({ text: "equipment" });
    expect(generic).toMatchObject({
      kind: "reject_generic",
      missingArchetypeFacts: [
        "equipment.material",
        "equipment.principal_function",
        "equipment.sector",
      ],
    });

    const service = gateDescriptionQuality({
      text: "training and consulting services for theatre equipment",
    });
    expect(service).toEqual({
      kind: "reject_service",
      reason: "service_description_not_physical_goods",
    });

    const unsafeBundle = gateDescriptionQuality({
      text: "first aid kit with sterile gloves, wipes and scissors",
    });
    expect(unsafeBundle).toMatchObject({
      kind: "reject_bundle_unsafe",
      markAs: "gir_3_candidate",
    });
    expect(unsafeBundle.reason).toContain("retail set");

    const safeBundle = gateDescriptionQuality({
      text: "bulk nitrile examination gloves and bulk paperboard packaging cartons",
    });
    expect(safeBundle).toMatchObject({ kind: "split_safe" });
    if (safeBundle.kind !== "split_safe") {
      throw new Error("expected safe bundle split");
    }
    expect(safeBundle.subDescriptions.map((item) => item.text)).toEqual([
      "bulk nitrile examination gloves",
      "bulk paperboard packaging cartons",
    ]);
  });

  it("L5-T-hmrc_fpo-010 runs normalized query waves across source families and counts each method once", () => {
    const verdict = gateDescriptionQuality({
      text: "sterile nitrile examination gloves bulk pack",
    });

    expect(verdict).toMatchObject({
      kind: "accept",
      attributes: {
        headNoun: "gloves",
        material: "nitrile",
        principalFunction: "examination protection",
        sector: "medical",
        sterilityMedical: "sterile",
        packForm: "bulk",
      },
    });
    if (verdict.kind !== "accept") {
      throw new Error("expected accepted glove description");
    }

    const waves = generateQueryWaves(verdict.attributes);
    expect(waves.map((wave) => wave.strategy)).toEqual([
      "head_noun",
      "material_head_noun",
      "function_head_noun",
      "sector_material_function_head_noun",
      "pack_form_material_head_noun",
    ]);
    expect(waves.map((wave) => wave.query_en)).toEqual([
      "gloves",
      "nitrile gloves",
      "examination protection gloves",
      "medical nitrile examination protection gloves",
      "bulk nitrile gloves",
    ]);

    const queryPlan = planRecallQueryWaves(verdict.attributes);
    expect(queryPlan).toHaveLength(
      waves.length * RECALL_QUERY_SOURCE_FAMILIES.length,
    );
    expect(countRecallSourceFamilies(queryPlan)).toBe(
      RECALL_QUERY_SOURCE_FAMILIES.length,
    );

    for (const sourceFamily of RECALL_QUERY_SOURCE_FAMILIES) {
      const sourceWaves = queryPlan.filter(
        (planned) => planned.sourceFamily === sourceFamily,
      );
      expect(sourceWaves).toHaveLength(waves.length);
      expect(new Set(sourceWaves.map((planned) => planned.familyCountKey))).toEqual(
        new Set([sourceFamily]),
      );
    }
  });
});

async function readCassette(name: string): Promise<CassetteFixture> {
  return (await Bun.file(
    new URL(`../../../../tests/fixtures/cassettes/l5/hmrc/${name}`, import.meta.url),
  ).json()) as CassetteFixture;
}

async function readCassetteBody(cassette: CassetteFixture): Promise<string> {
  return Bun.file(
    new URL(
      `../../../../tests/fixtures/cassettes/l5/hmrc/${cassette.response.body_path}`,
      import.meta.url,
    ),
  ).text();
}

async function bodySha256(relativePath: string): Promise<string> {
  const body = await Bun.file(
    new URL(
      `../../../../tests/fixtures/cassettes/l5/hmrc/${relativePath}`,
      import.meta.url,
    ),
  ).arrayBuffer();
  return `sha256:${createHash("sha256").update(Buffer.from(body)).digest("hex")}`;
}

function acceptedAttributes(text: string) {
  const verdict = gateDescriptionQuality({ text });
  if (verdict.kind !== "accept") {
    throw new Error(`expected accepted description, received ${verdict.kind}`);
  }

  return verdict.attributes;
}

function recallCandidate(
  code: string,
  title: string,
  sourceFamily: (typeof RECALL_SOURCE_FAMILY_ORDER)[number],
) {
  return createRecallCandidate({
    code,
    title,
    sourceFamily,
    retrievalScoreBps: 8_000,
  });
}
