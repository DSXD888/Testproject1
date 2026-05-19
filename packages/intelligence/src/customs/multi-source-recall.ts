export const FPO_ACCESS_UNAVAILABLE = "FPO_ACCESS_UNAVAILABLE";

export type RecallSourceFamily =
  | "hmrc_fpo_or_search_suggestion"
  | "hmrc_tariff_keyword_search"
  | "cpv_chapter_prior"
  | "rule_based_chapter_hint"
  | "atr_codebook_reuse"
  | "taric_cross_check";

export type RecallSourceStatus =
  | "available"
  | "unavailable"
  | "malformed"
  | "restricted"
  | "declined";

export interface RecallCandidate {
  commodityCode: string;
  referencedClass: string;
  title: string;
  sourceFamily: RecallSourceFamily;
  relevanceBps: number;
  legalEvidence: false;
  authorityRole: "retrieval_relevance_only" | "advisory_cross_check";
  sourceUrl: string;
}

export interface RecallSourceResult {
  sourceFamily: RecallSourceFamily;
  status: RecallSourceStatus;
  candidates: RecallCandidate[];
  failureCode?: typeof FPO_ACCESS_UNAVAILABLE;
  failureReason?: string;
  continuationAllowed: boolean;
  legalEvidence: false;
  authorityRole: "retrieval_relevance_only" | "advisory_cross_check";
  staleClassification: boolean;
  guessingAllowed: false;
}

export interface HmrcEndpointDecision {
  checkedAt: string;
  fpoEndpoint: {
    url: "https://search.trade-tariff.service.gov.uk/fpo-code-search";
    status: "restricted_requires_invitation_or_api_key" | "network_unavailable";
    failureCode: typeof FPO_ACCESS_UNAVAILABLE;
    procurementActionCreated: false;
  };
  publicReplacement: {
    url: "https://www.trade-tariff.service.gov.uk/api/v2/search_references";
    status: "available" | "unavailable";
    role: "retrieval_relevance_only";
  };
}

export interface TaricAdvisoryDivergence {
  code: "TARIC_ADVISORY_DIVERGENCE";
  hmrcAuthorityPreserved: true;
  taricRole: "advisory_cross_check";
  affectedMeasureKind: string;
  reason: string;
}

export interface RecallCoverageEvaluation {
  green: boolean;
  activeSources: RecallSourceFamily[];
  declinedSources: string[];
  confidencePenaltyBps: number;
  notes: string[];
}

interface HmrcSearchReferenceAttributes {
  title?: unknown;
  referenced_id?: unknown;
  referenced_class?: unknown;
  goods_nomenclature_item_id?: unknown;
}

interface HmrcSearchReferenceItem {
  id?: unknown;
  type?: unknown;
  attributes?: HmrcSearchReferenceAttributes;
}

interface HmrcSearchReferenceBody {
  data?: unknown;
}

export function parseHmrcSearchReferenceRecall(
  bodyText: string,
  opts: {
    searchTerm: string;
    sourceUrl: string;
  },
): RecallSourceResult {
  let parsed: HmrcSearchReferenceBody;
  try {
    parsed = JSON.parse(bodyText) as HmrcSearchReferenceBody;
  } catch {
    return fpoUnavailable("hmrc_search_reference_json_parse_failed", "malformed");
  }

  if (!Array.isArray(parsed.data)) {
    return fpoUnavailable("hmrc_search_reference_data_missing", "malformed");
  }

  const tokens = tokenize(opts.searchTerm);
  const candidates = parsed.data
    .flatMap((item) => candidateFromSearchReference(item, tokens, opts.sourceUrl))
    .sort((left, right) => right.relevanceBps - left.relevanceBps);

  if (candidates.length === 0) {
    return {
      sourceFamily: "hmrc_fpo_or_search_suggestion",
      status: "unavailable",
      candidates: [],
      failureCode: FPO_ACCESS_UNAVAILABLE,
      failureReason: "hmrc_search_reference_no_candidate_match",
      continuationAllowed: true,
      legalEvidence: false,
      authorityRole: "retrieval_relevance_only",
      staleClassification: false,
      guessingAllowed: false,
    };
  }

  return {
    sourceFamily: "hmrc_fpo_or_search_suggestion",
    status: "available",
    candidates,
    continuationAllowed: true,
    legalEvidence: false,
    authorityRole: "retrieval_relevance_only",
    staleClassification: false,
    guessingAllowed: false,
  };
}

export function interpretFpoHttpResponse(opts: {
  status: number;
  bodyText: string;
}): RecallSourceResult {
  if (opts.status === 401 || opts.status === 403) {
    return {
      ...fpoUnavailable(`hmrc_fpo_http_${opts.status}`, "restricted"),
      staleClassification: true,
    };
  }

  if (opts.status === 429 || opts.status >= 500) {
    return {
      ...fpoUnavailable(`hmrc_fpo_http_${opts.status}`, "unavailable"),
      staleClassification: true,
    };
  }

  if (opts.status < 200 || opts.status >= 300) {
    return fpoUnavailable(`hmrc_fpo_http_${opts.status}`, "unavailable");
  }

  try {
    const parsed = JSON.parse(opts.bodyText) as { results?: unknown };
    if (!Array.isArray(parsed.results)) {
      return fpoUnavailable("hmrc_fpo_results_missing", "malformed");
    }
  } catch {
    return fpoUnavailable("hmrc_fpo_json_parse_failed", "malformed");
  }

  return {
    sourceFamily: "hmrc_fpo_or_search_suggestion",
    status: "available",
    candidates: [],
    continuationAllowed: true,
    legalEvidence: false,
    authorityRole: "retrieval_relevance_only",
    staleClassification: false,
    guessingAllowed: false,
  };
}

export async function probeHmrcFpoEndpointDecision(opts: {
  checkedAt: string;
  fetcher?: typeof fetch;
}): Promise<HmrcEndpointDecision> {
  const fetcher = opts.fetcher ?? fetch;
  const publicReplacementUrl =
    "https://www.trade-tariff.service.gov.uk/api/v2/search_references?query%5Bletter%5D=s";
  let publicReplacementStatus: "available" | "unavailable" = "unavailable";
  let fpoStatus: "restricted_requires_invitation_or_api_key" | "network_unavailable" =
    "network_unavailable";

  try {
    const publicResponse = await fetcher(publicReplacementUrl, {
      headers: { accept: "application/json" },
    });
    publicReplacementStatus = publicResponse.ok ? "available" : "unavailable";
  } catch {
    publicReplacementStatus = "unavailable";
  }

  try {
    const fpoResponse = await fetcher(
      "https://search.trade-tariff.service.gov.uk/fpo-code-search",
      {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({ description: "medical nitrile gloves", digits: 10 }),
      },
    );
    fpoStatus =
      fpoResponse.status === 401 || fpoResponse.status === 403
        ? "restricted_requires_invitation_or_api_key"
        : "network_unavailable";
  } catch {
    fpoStatus = "network_unavailable";
  }

  return {
    checkedAt: opts.checkedAt,
    fpoEndpoint: {
      url: "https://search.trade-tariff.service.gov.uk/fpo-code-search",
      status: fpoStatus,
      failureCode: FPO_ACCESS_UNAVAILABLE,
      procurementActionCreated: false,
    },
    publicReplacement: {
      url: "https://www.trade-tariff.service.gov.uk/api/v2/search_references",
      status: publicReplacementStatus,
      role: "retrieval_relevance_only",
    },
  };
}

export function reconcileHmrcWithTaric(opts: {
  hmrcCommodityCode: string;
  taricCommodityCode: string;
  affectedMeasureKind: string;
  taricCandidateTitle: string;
  sourceUrl: string;
}): {
  authoritativeCommodityCode: string;
  advisoryCandidates: RecallCandidate[];
  divergences: TaricAdvisoryDivergence[];
} {
  const advisoryCandidate: RecallCandidate = {
    commodityCode: opts.taricCommodityCode,
    referencedClass: "TARIC",
    title: opts.taricCandidateTitle,
    sourceFamily: "taric_cross_check",
    relevanceBps: 0,
    legalEvidence: false,
    authorityRole: "advisory_cross_check",
    sourceUrl: opts.sourceUrl,
  };

  if (opts.hmrcCommodityCode === opts.taricCommodityCode) {
    return {
      authoritativeCommodityCode: opts.hmrcCommodityCode,
      advisoryCandidates: [advisoryCandidate],
      divergences: [],
    };
  }

  return {
    authoritativeCommodityCode: opts.hmrcCommodityCode,
    advisoryCandidates: [advisoryCandidate],
    divergences: [
      {
        code: "TARIC_ADVISORY_DIVERGENCE",
        hmrcAuthorityPreserved: true,
        taricRole: "advisory_cross_check",
        affectedMeasureKind: opts.affectedMeasureKind,
        reason:
          "TARIC disagreement is retained as evidence divergence only; HMRC remains authoritative for UK national measures.",
      },
    ],
  };
}

export function evaluateRecallFamilyCoverage(opts: {
  sourceResults: RecallSourceResult[];
  declinedSources?: string[];
}): RecallCoverageEvaluation {
  const activeSources = opts.sourceResults
    .filter((result) => result.status === "available")
    .map((result) => result.sourceFamily);
  const uniqueActiveSources = [...new Set(activeSources)];
  const requiredNonFpoFamilies: RecallSourceFamily[] = [
    "hmrc_tariff_keyword_search",
    "cpv_chapter_prior",
    "rule_based_chapter_hint",
    "atr_codebook_reuse",
  ];
  const missing = requiredNonFpoFamilies.filter(
    (family) => !uniqueActiveSources.includes(family),
  );
  const unavailableCount = opts.sourceResults.filter(
    (result) => result.status !== "available",
  ).length;

  return {
    green: missing.length === 0,
    activeSources: uniqueActiveSources,
    declinedSources: opts.declinedSources ?? [],
    confidencePenaltyBps: unavailableCount * 250,
    notes:
      missing.length === 0
        ? ["FPO unavailable is tolerated because four non-FPO recall families are available."]
        : [`Missing recall families: ${missing.join(", ")}`],
  };
}

export function availableRecallSource(sourceFamily: RecallSourceFamily): RecallSourceResult {
  return {
    sourceFamily,
    status: "available",
    candidates: [],
    continuationAllowed: true,
    legalEvidence: false,
    authorityRole:
      sourceFamily === "taric_cross_check"
        ? "advisory_cross_check"
        : "retrieval_relevance_only",
    staleClassification: false,
    guessingAllowed: false,
  };
}

function candidateFromSearchReference(
  item: unknown,
  queryTokens: string[],
  sourceUrl: string,
): RecallCandidate[] {
  if (!isSearchReferenceItem(item)) {
    return [];
  }

  const attributes = item.attributes;
  if (attributes === undefined) {
    return [];
  }

  const title = asString(attributes.title);
  const commodityCode = asString(attributes.goods_nomenclature_item_id);
  const referencedClass = asString(attributes.referenced_class);
  if (title === null || commodityCode === null || referencedClass === null) {
    return [];
  }

  const titleTokens = tokenize(title);
  const overlap = queryTokens.filter((token) => titleTokens.includes(token)).length;
  if (overlap === 0) {
    return [];
  }

  const relevanceBps = Math.min(
    10_000,
    Math.round((overlap / Math.max(queryTokens.length, 1)) * 10_000),
  );

  return [
    {
      commodityCode,
      referencedClass,
      title,
      sourceFamily: "hmrc_fpo_or_search_suggestion",
      relevanceBps,
      legalEvidence: false,
      authorityRole: "retrieval_relevance_only",
      sourceUrl,
    },
  ];
}

function fpoUnavailable(
  failureReason: string,
  status: Extract<RecallSourceStatus, "unavailable" | "malformed" | "restricted">,
): RecallSourceResult {
  return {
    sourceFamily: "hmrc_fpo_or_search_suggestion",
    status,
    candidates: [],
    failureCode: FPO_ACCESS_UNAVAILABLE,
    failureReason,
    continuationAllowed: true,
    legalEvidence: false,
    authorityRole: "retrieval_relevance_only",
    staleClassification: status !== "malformed",
    guessingAllowed: false,
  };
}

function isSearchReferenceItem(value: unknown): value is HmrcSearchReferenceItem {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const maybeItem = value as HmrcSearchReferenceItem;
  return maybeItem.type === "search_reference" && typeof maybeItem.attributes === "object";
}

function asString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function tokenize(value: string): string[] {
  return value
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((token) => token.length > 2);
}
