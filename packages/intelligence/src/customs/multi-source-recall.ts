export const FPO_ACCESS_UNAVAILABLE = "FPO_ACCESS_UNAVAILABLE";

export {
  InMemoryRecallLatticeStore,
  InMemoryTariffGraph,
  expandRecallLatticeToTariffGraph,
  persistRecallLattice,
  recallLatticeRowsFromExpansion,
  verifyShortlistedTariffBranches,
} from "./ten-digit-resolution";
export type {
  AdditionalCodeRequirement,
  ExpandedTariffCandidate,
  ProductCommodityCodeOutboxEntry,
  ProductCommodityCodeRow,
  RecallLatticePersistenceCandidate,
  RecallLatticePersistenceStore,
  TariffBranchVerification,
  TariffFamilySummary,
  TariffGraph,
  TariffGraphExpansionResult,
  TariffMeasure,
  TariffNode,
  TariffNodeLevel,
} from "./ten-digit-resolution";

export type RecallSourceFamily =
  | "hmrc_fpo_or_search_suggestion"
  | "hmrc_tariff_keyword_search"
  | "cpv_chapter_prior"
  | "rule_based_chapter_hint"
  | "atr_codebook_reuse"
  | "taric_cross_check";

export type SourceFamily = Exclude<RecallSourceFamily, "taric_cross_check">;

export const RECALL_SOURCE_FAMILY_ORDER: SourceFamily[] = [
  "hmrc_fpo_or_search_suggestion",
  "cpv_chapter_prior",
  "rule_based_chapter_hint",
  "hmrc_tariff_keyword_search",
  "atr_codebook_reuse",
];

export type RecallSourceStatus =
  | "available"
  | "unavailable"
  | "malformed"
  | "restricted"
  | "declined";

export type RecallCandidateLevel = "heading" | "subheading" | "cn8";
export type RecallSourceFailureMode =
  | "rate_limited"
  | "downtime"
  | "schema_drift"
  | "access_restricted"
  | "none";

export interface RecallCandidate {
  code: string;
  level: RecallCandidateLevel;
  retrievalScoreBps: number;
  commodityCode: string;
  referencedClass: string;
  title: string;
  sourceFamily: RecallSourceFamily;
  relevanceBps: number;
  legalEvidence: false;
  authorityRole: "retrieval_relevance_only" | "advisory_cross_check";
  sourceUrl: string;
}

export interface SourceFamilyResult {
  sourceFamily: SourceFamily;
  state: "agreed" | "diverged" | "unavailable";
  candidates: RecallCandidate[];
  scoreBps: number | null;
  failureMode?: RecallSourceFailureMode;
}

export interface CandidateRecallAgreement {
  candidateCode: string;
  agreement: SourceFamilyResult[];
}

export interface MultiSourceRecallResult {
  attributes: import("./description-quality").ExtractedAttributes;
  perSourceResults: SourceFamilyResult[];
  candidateRecallAgreement: CandidateRecallAgreement[];
  independentSourceFamiliesAvailable: number;
  independentSourceFamiliesAgreeing: number;
  classificationBlockers: Array<"CLASSIFICATION_AMBIGUOUS">;
  autoClassificationAllowed: boolean;
}

export type RecallSourceAdapter = (opts: {
  attributes: import("./description-quality").ExtractedAttributes;
  cpvCode?: string;
  sourceFamily: SourceFamily;
}) => Promise<RecallCandidate[]> | RecallCandidate[];

export type RecallSourceAdapters = Partial<Record<SourceFamily, RecallSourceAdapter>>;

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

interface HmrcTariffSearchAttributes {
  description?: unknown;
  formatted_description?: unknown;
  goods_nomenclature_item_id?: unknown;
  goods_nomenclature_sid?: unknown;
}

interface HmrcTariffSearchItem {
  id?: unknown;
  type?: unknown;
  attributes?: HmrcTariffSearchAttributes;
}

interface HmrcTariffSearchBody {
  data?: unknown;
}

export class MultiSourceRecall {
  constructor(private readonly adapters: RecallSourceAdapters = {}) {}

  async run(
    attributes: import("./description-quality").ExtractedAttributes,
    cpvCode?: string,
  ): Promise<MultiSourceRecallResult> {
    const perSourceResults = await Promise.all(
      RECALL_SOURCE_FAMILY_ORDER.map((sourceFamily) =>
        this.runSourceFamily(sourceFamily, attributes, cpvCode),
      ),
    );

    const candidateRecallAgreement = buildCandidateRecallAgreement(perSourceResults);
    const topAgreement = candidateRecallAgreement[0]?.agreement ?? [];
    const independentSourceFamiliesAvailable = perSourceResults.filter(
      (result) => result.state !== "unavailable",
    ).length;
    const independentSourceFamiliesAgreeing = topAgreement.filter(
      (result) => result.state === "agreed",
    ).length;
    const classificationBlockers = hasMaterialHs6Split(perSourceResults)
      ? (["CLASSIFICATION_AMBIGUOUS"] as const)
      : [];

    return {
      attributes,
      perSourceResults,
      candidateRecallAgreement,
      independentSourceFamiliesAvailable,
      independentSourceFamiliesAgreeing,
      classificationBlockers: [...classificationBlockers],
      autoClassificationAllowed:
        classificationBlockers.length === 0 &&
        independentSourceFamiliesAvailable >= 2 &&
        independentSourceFamiliesAgreeing >= 2,
    };
  }

  async runSourceFamily(
    sourceFamily: SourceFamily,
    attributes: import("./description-quality").ExtractedAttributes,
    cpvCode?: string,
  ): Promise<SourceFamilyResult> {
    const adapter = this.adapters[sourceFamily];
    if (adapter === undefined) {
      return unavailableSourceFamily(sourceFamily, "none");
    }

    try {
      const candidates = await adapter({ attributes, cpvCode, sourceFamily });
      if (candidates.length === 0) {
        return unavailableSourceFamily(sourceFamily, "none");
      }

      return {
        sourceFamily,
        state: "agreed",
        candidates,
        scoreBps: candidates[0]?.retrievalScoreBps ?? null,
        failureMode: "none",
      };
    } catch (error) {
      return unavailableSourceFamily(sourceFamily, sourceFailureMode(error));
    }
  }
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

export function parseHmrcTariffKeywordSearchRecall(
  bodyText: string,
  opts: {
    searchTerm: string;
    sourceUrl: string;
  },
): SourceFamilyResult {
  let parsed: HmrcTariffSearchBody;
  try {
    parsed = JSON.parse(bodyText) as HmrcTariffSearchBody;
  } catch {
    return unavailableSourceFamily("hmrc_tariff_keyword_search", "schema_drift");
  }

  if (!Array.isArray(parsed.data)) {
    return unavailableSourceFamily("hmrc_tariff_keyword_search", "schema_drift");
  }

  const tokens = tokenize(opts.searchTerm);
  const candidates = parsed.data
    .flatMap((item) => candidateFromTariffSearch(item, tokens, opts.sourceUrl))
    .sort((left, right) => right.retrievalScoreBps - left.retrievalScoreBps);

  if (candidates.length === 0) {
    return unavailableSourceFamily("hmrc_tariff_keyword_search", "none");
  }

  return {
    sourceFamily: "hmrc_tariff_keyword_search",
    state: "agreed",
    candidates,
    scoreBps: candidates[0]?.retrievalScoreBps ?? null,
    failureMode: "none",
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
    code: opts.taricCommodityCode,
    level: inferCandidateLevel(opts.taricCommodityCode),
    retrievalScoreBps: 0,
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

export function createRecallCandidate(opts: {
  code: string;
  title: string;
  sourceFamily: SourceFamily;
  retrievalScoreBps: number;
  sourceUrl?: string;
  referencedClass?: string;
}): RecallCandidate {
  return {
    code: opts.code,
    level: inferCandidateLevel(opts.code),
    retrievalScoreBps: opts.retrievalScoreBps,
    commodityCode: opts.code,
    referencedClass: opts.referencedClass ?? "goods_nomenclature",
    title: opts.title,
    sourceFamily: opts.sourceFamily,
    relevanceBps: opts.retrievalScoreBps,
    legalEvidence: false,
    authorityRole: "retrieval_relevance_only",
    sourceUrl: opts.sourceUrl ?? "local://l5-recall-fixture",
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
      code: commodityCode,
      level: inferCandidateLevel(commodityCode),
      retrievalScoreBps: relevanceBps,
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

function candidateFromTariffSearch(
  item: unknown,
  queryTokens: string[],
  sourceUrl: string,
): RecallCandidate[] {
  if (!isTariffSearchItem(item)) {
    return [];
  }

  const attributes = item.attributes;
  if (attributes === undefined) {
    return [];
  }

  const title =
    asString(attributes.description) ??
    asString(attributes.formatted_description) ??
    "HMRC tariff keyword result";
  const commodityCode =
    asString(attributes.goods_nomenclature_item_id) ??
    asString(attributes.goods_nomenclature_sid) ??
    asString(item.id);
  if (commodityCode === null) {
    return [];
  }

  const titleTokens = tokenize(title);
  const overlap = queryTokens.filter((token) => titleTokens.includes(token)).length;
  if (overlap === 0) {
    return [];
  }

  const retrievalScoreBps = Math.min(
    10_000,
    Math.round((overlap / Math.max(queryTokens.length, 1)) * 10_000),
  );

  return [
    createRecallCandidate({
      code: commodityCode,
      title,
      sourceFamily: "hmrc_tariff_keyword_search",
      retrievalScoreBps,
      sourceUrl,
    }),
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

function buildCandidateRecallAgreement(
  perSourceResults: SourceFamilyResult[],
): CandidateRecallAgreement[] {
  const candidateCodes = new Set(
    perSourceResults.flatMap((result) =>
      result.candidates.map((candidate) => candidate.code),
    ),
  );

  return [...candidateCodes]
    .map((candidateCode) => ({
      candidateCode,
      agreement: RECALL_SOURCE_FAMILY_ORDER.map((sourceFamily) =>
        agreementForSource(candidateCode, sourceFamily, perSourceResults),
      ),
    }))
    .sort((left, right) => supportScore(right) - supportScore(left));
}

function agreementForSource(
  candidateCode: string,
  sourceFamily: SourceFamily,
  perSourceResults: SourceFamilyResult[],
): SourceFamilyResult {
  const sourceResult = perSourceResults.find(
    (result) => result.sourceFamily === sourceFamily,
  );
  if (sourceResult === undefined || sourceResult.candidates.length === 0) {
    return unavailableSourceFamily(sourceFamily, sourceResult?.failureMode ?? "none");
  }

  const state = sourceResult.candidates.some((candidate) =>
    sameHs6Family(candidate.code, candidateCode),
  )
    ? "agreed"
    : "diverged";

  return {
    ...sourceResult,
    state,
  };
}

function supportScore(candidate: CandidateRecallAgreement): number {
  return candidate.agreement.reduce((score, sourceResult) => {
    if (sourceResult.state !== "agreed") {
      return score;
    }

    return score + (sourceResult.scoreBps ?? 0);
  }, 0);
}

function hasMaterialHs6Split(perSourceResults: SourceFamilyResult[]): boolean {
  const hs6Families = new Set(
    perSourceResults.flatMap((result) =>
      result.candidates.map((candidate) => hs6Family(candidate.code)),
    ),
  );
  return hs6Families.size > 1;
}

function unavailableSourceFamily(
  sourceFamily: SourceFamily,
  failureMode: RecallSourceFailureMode,
): SourceFamilyResult {
  return {
    sourceFamily,
    state: "unavailable",
    candidates: [],
    scoreBps: null,
    failureMode,
  };
}

function sourceFailureMode(error: unknown): RecallSourceFailureMode {
  if (error instanceof Error) {
    if (error.message.includes("429") || error.message.includes("rate")) {
      return "rate_limited";
    }

    if (error.message.includes("schema")) {
      return "schema_drift";
    }

    if (error.message.includes("restricted") || error.message.includes("403")) {
      return "access_restricted";
    }
  }

  return "downtime";
}

function isSearchReferenceItem(value: unknown): value is HmrcSearchReferenceItem {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const maybeItem = value as HmrcSearchReferenceItem;
  return maybeItem.type === "search_reference" && typeof maybeItem.attributes === "object";
}

function isTariffSearchItem(value: unknown): value is HmrcTariffSearchItem {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const maybeItem = value as HmrcTariffSearchItem;
  return typeof maybeItem.attributes === "object";
}

function inferCandidateLevel(code: string): RecallCandidateLevel {
  if (code.length <= 4) {
    return "heading";
  }

  if (code.length <= 6) {
    return "subheading";
  }

  return "cn8";
}

function sameHs6Family(left: string, right: string): boolean {
  return hs6Family(left) === hs6Family(right);
}

function hs6Family(code: string): string {
  return code.slice(0, 6);
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
