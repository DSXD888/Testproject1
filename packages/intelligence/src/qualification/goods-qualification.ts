export type LegalCategory = "goods" | "services" | "works" | "mixed" | "unknown";
export type GoodsSeparability = boolean | "uncertain";
export type QualificationBasis = "notice_only" | "notice_plus_docs" | "lot_level";
export type AccessRoute =
  | "open_tender"
  | "restricted_tender"
  | "framework_call_off"
  | "dynamic_market"
  | "direct_award"
  | "closed_framework_award"
  | "not_bidable";
export type QualificationDecision = "pass" | "review" | "reject";

export interface GoodsDensitySignals {
  physicalGoodsMentions?: number;
  serviceMentions?: number;
  quantityMentions?: number;
  attachmentSignals?: number;
}

export interface GoodsQualificationInput {
  cpvCodes: string[];
  title?: string | null;
  description?: string | null;
  descriptionText?: string | null;
  tenderDeadline?: string | null;
  now?: string | Date;
  procedureType?: string | null;
  accessRoute?: AccessRoute | null;
  qualificationBasis: QualificationBasis;
  totalValuePence?: number | null;
  goodsAddressableValuePence?: number | null;
  goodsSeparability?: GoodsSeparability | null;
  goodsDensitySignals?: GoodsDensitySignals;
  goodsConfidenceBps?: number | null;
  rationale?: string[];
}

export interface GoodsQualificationOutput {
  expiryQualified: boolean;
  goodsQualified: boolean;
  goodsConfidenceBps: number;
  legalCategory: LegalCategory;
  goodsDensityScore: number;
  goodsSeparability: GoodsSeparability;
  qualificationBasis: QualificationBasis;
  addressableValuePence: number | null;
  accessRoute: AccessRoute;
  qualificationResult: QualificationDecision;
  exclusionReasons: string[];
  rationale: string[];
}

const REVIEWABLE_ACCESS_ROUTES = new Set<AccessRoute>([
  "restricted_tender",
  "framework_call_off",
]);

const REJECTED_ACCESS_ROUTES = new Set<AccessRoute>([
  "direct_award",
  "closed_framework_award",
  "not_bidable",
]);

const GOODS_DOMINANCE_FLOOR_BPS = 6000;
const GOODS_PRESENCE_FLOOR_BPS = 3500;
const NO_LIST_HIGH_CONFIDENCE_BPS = 8000;
const OUT_OF_SCOPE_BESPOKE = "out_of_scope_bespoke";
const OUT_OF_SCOPE_NO_LIST = "out_of_scope_no_list";

interface ScopeExclusionFlag {
  reason: typeof OUT_OF_SCOPE_BESPOKE | typeof OUT_OF_SCOPE_NO_LIST;
  disposition: QualificationDecision;
  evidence: string;
}

export function qualifyGoodsOpportunity(
  input: GoodsQualificationInput,
): GoodsQualificationOutput {
  const legalCategory = classifyLegalCategory(input.cpvCodes);
  const accessRoute = classifyAccessRoute(input);
  const expiryVerdict = evaluateExpiry({
    accessRoute,
    now: input.now,
    tenderDeadline: input.tenderDeadline,
  });
  const goodsSeparability = normalizeGoodsSeparability(
    legalCategory,
    input.goodsSeparability,
  );
  const goodsDensityScore = computeGoodsDensityScore({
    legalCategory,
    goodsDensitySignals: input.goodsDensitySignals,
  });
  const goodsConfidenceBps = Math.min(
    10_000,
    input.goodsConfidenceBps ?? Math.round(goodsDensityScore * 10_000),
  );
  const rationale = [...(input.rationale ?? [])];

  if (expiryVerdict === "review") {
    rationale.push("Tender deadline is missing or could not be normalized.");
  } else if (expiryVerdict === "reject" && accessRoute !== "dynamic_market") {
    rationale.push("Tender deadline is inside the 30-day qualification window.");
  }

  if (REJECTED_ACCESS_ROUTES.has(accessRoute)) {
    rationale.push(`Access route ${accessRoute} is not actionable for the operator.`);
  } else if (REVIEWABLE_ACCESS_ROUTES.has(accessRoute)) {
    rationale.push(`Access route ${accessRoute} requires operator review before bidding.`);
  }

  if (goodsConfidenceBps < 6000) {
    rationale.push("Goods qualification confidence remains below the autonomous threshold.");
  }

  const addressableValuePence = computeAddressableValue({
    legalCategory,
    goodsAddressableValuePence: input.goodsAddressableValuePence,
    goodsSeparability,
    totalValuePence: input.totalValuePence,
  });
  const exclusionFlags = detectScopeExclusionFlags({
    cpvCodes: input.cpvCodes,
    description: input.description,
    descriptionText: input.descriptionText,
    goodsConfidenceBps,
    title: input.title,
  });

  for (const flag of exclusionFlags) {
    rationale.push(`${flag.reason}: ${flag.evidence}`);
  }

  const qualificationResult = determineQualificationDecision({
    legalCategory,
    accessRoute,
    expiryVerdict,
    goodsDensityScore,
    goodsConfidenceBps,
    goodsSeparability,
    addressableValuePence,
    totalValuePence: input.totalValuePence,
    exclusionFlags,
  });

  return {
    expiryQualified: expiryVerdict === "pass",
    goodsQualified: qualificationResult !== "reject",
    goodsConfidenceBps,
    legalCategory,
    goodsDensityScore,
    goodsSeparability,
    qualificationBasis: input.qualificationBasis,
    addressableValuePence,
    accessRoute,
    qualificationResult,
    exclusionReasons: exclusionFlags.map((flag) => flag.reason),
    rationale,
  };
}

export function classifyLegalCategory(cpvCodes: string[]): LegalCategory {
  const categories = new Set<LegalCategory>();

  for (const rawCode of cpvCodes) {
    const code = rawCode.replaceAll(/[^0-9]/g, "");
    if (code.length < 2) {
      continue;
    }

    const division = Number.parseInt(code.slice(0, 2), 10);
    if (Number.isNaN(division)) {
      continue;
    }

    categories.add(categoryForDivision(division));
  }

  categories.delete("unknown");

  if (categories.size === 0) {
    return "unknown";
  }

  if (categories.size > 1) {
    return "mixed";
  }

  return [...categories][0] ?? "unknown";
}

export function classifyAccessRoute(
  input: Pick<GoodsQualificationInput, "accessRoute" | "procedureType">,
): AccessRoute {
  if (input.accessRoute) {
    return input.accessRoute;
  }

  const procedureType = input.procedureType?.trim().toLowerCase();
  if (!procedureType) {
    return "open_tender";
  }

  if (procedureType.includes("dynamic") || procedureType.includes("dps")) {
    return "dynamic_market";
  }

  if (procedureType.includes("direct award")) {
    return "direct_award";
  }

  if (procedureType.includes("closed framework")) {
    return "closed_framework_award";
  }

  if (procedureType.includes("framework")) {
    return "framework_call_off";
  }

  if (procedureType.includes("restricted")) {
    return "restricted_tender";
  }

  if (procedureType.includes("open")) {
    return "open_tender";
  }

  return "not_bidable";
}

export function evaluateExpiry(input: {
  tenderDeadline?: string | null;
  now?: string | Date;
  accessRoute: AccessRoute;
}): QualificationDecision {
  if (input.accessRoute === "dynamic_market") {
    return "pass";
  }

  if (!input.tenderDeadline) {
    return "review";
  }

  const deadline = new Date(input.tenderDeadline);
  const now = new Date(input.now ?? new Date());
  if (Number.isNaN(deadline.getTime()) || Number.isNaN(now.getTime())) {
    return "review";
  }

  const differenceMs = deadline.getTime() - now.getTime();
  const minimumLeadMs = 30 * 24 * 60 * 60 * 1000;
  return differenceMs >= minimumLeadMs ? "pass" : "reject";
}

function categoryForDivision(division: number): LegalCategory {
  if (division === 45) {
    return "works";
  }

  if (division >= 14 && division <= 44) {
    return "goods";
  }

  if (division >= 50 && division <= 98) {
    return "services";
  }

  return "unknown";
}

function normalizeGoodsSeparability(
  legalCategory: LegalCategory,
  goodsSeparability: GoodsSeparability | null | undefined,
): GoodsSeparability {
  if (goodsSeparability !== null && goodsSeparability !== undefined) {
    return goodsSeparability;
  }

  if (legalCategory === "goods") {
    return true;
  }

  if (legalCategory === "services") {
    return false;
  }

  if (legalCategory === "works") {
    return false;
  }

  return "uncertain";
}

function computeGoodsDensityScore(input: {
  legalCategory: LegalCategory;
  goodsDensitySignals?: GoodsDensitySignals;
}): number {
  const baseScore =
    input.legalCategory === "goods"
      ? 0.85
      : input.legalCategory === "mixed"
        ? 0.6
        : input.legalCategory === "works"
          ? 0.3
          : input.legalCategory === "services"
            ? 0.15
            : 0.45;

  const signals = input.goodsDensitySignals ?? {};
  const adjustedScore =
    baseScore +
    (signals.physicalGoodsMentions ?? 0) * 0.05 +
    (signals.quantityMentions ?? 0) * 0.04 +
    (signals.attachmentSignals ?? 0) * 0.03 -
    (signals.serviceMentions ?? 0) * 0.08;

  return clamp(adjustedScore, 0, 1);
}

function computeAddressableValue(input: {
  legalCategory: LegalCategory;
  totalValuePence?: number | null;
  goodsAddressableValuePence?: number | null;
  goodsSeparability: GoodsSeparability;
}): number | null {
  if (input.legalCategory === "goods") {
    return input.totalValuePence ?? null;
  }

  if (
    input.legalCategory === "mixed" &&
    input.goodsSeparability !== false &&
    input.goodsAddressableValuePence !== undefined
  ) {
    return input.goodsAddressableValuePence ?? null;
  }

  if (
    input.legalCategory === "works" &&
    input.goodsSeparability !== false &&
    input.goodsAddressableValuePence !== undefined
  ) {
    return input.goodsAddressableValuePence ?? null;
  }

  return null;
}

function determineQualificationDecision(input: {
  legalCategory: LegalCategory;
  goodsDensityScore: number;
  goodsConfidenceBps: number;
  goodsSeparability: GoodsSeparability;
  addressableValuePence: number | null;
  totalValuePence?: number | null;
  exclusionFlags: ScopeExclusionFlag[];
  accessRoute: AccessRoute;
  expiryVerdict: QualificationDecision;
}): QualificationDecision {
  if (REJECTED_ACCESS_ROUTES.has(input.accessRoute)) {
    return "reject";
  }

  if (input.expiryVerdict === "reject") {
    return "reject";
  }

  if (input.expiryVerdict === "review") {
    return "review";
  }

  if (input.exclusionFlags.some((flag) => flag.disposition === "reject")) {
    return "reject";
  }

  if (input.exclusionFlags.some((flag) => flag.disposition === "review")) {
    return "review";
  }

  if (input.goodsConfidenceBps < 6000) {
    return "review";
  }

  if (input.legalCategory === "goods") {
    return REVIEWABLE_ACCESS_ROUTES.has(input.accessRoute) ? "review" : "pass";
  }

  if (
    input.legalCategory === "mixed" &&
    input.goodsSeparability === true
  ) {
    const goodsValueShareBps = computeValueShareBps(
      input.addressableValuePence,
      input.totalValuePence,
    );
    if (goodsValueShareBps !== null && goodsValueShareBps >= GOODS_DOMINANCE_FLOOR_BPS) {
      return REVIEWABLE_ACCESS_ROUTES.has(input.accessRoute) ? "review" : "pass";
    }

    return "review";
  }

  if (
    (input.legalCategory === "mixed" || input.legalCategory === "works") &&
    input.goodsSeparability !== false &&
    Math.round(input.goodsDensityScore * 10_000) >= GOODS_PRESENCE_FLOOR_BPS
  ) {
    return "review";
  }

  if (
    (input.legalCategory === "mixed" || input.legalCategory === "unknown") &&
    input.goodsDensityScore >= 0.55 &&
    input.goodsSeparability !== false
  ) {
    return "review";
  }

  if (
    input.legalCategory === "works" &&
    input.goodsDensityScore >= 0.7 &&
    input.goodsSeparability !== false
  ) {
    return "review";
  }

  return "reject";
}

function computeValueShareBps(
  addressableValuePence: number | null,
  totalValuePence?: number | null,
): number | null {
  if (
    addressableValuePence === null ||
    totalValuePence === null ||
    totalValuePence === undefined ||
    addressableValuePence < 0 ||
    totalValuePence <= 0
  ) {
    return null;
  }

  return Math.round((addressableValuePence * 10_000) / totalValuePence);
}

function detectScopeExclusionFlags(input: {
  cpvCodes: string[];
  title?: string | null;
  description?: string | null;
  descriptionText?: string | null;
  goodsConfidenceBps: number;
}): ScopeExclusionFlag[] {
  const flags: ScopeExclusionFlag[] = [];
  const noListFamily = detectNoListCpvFamily(input.cpvCodes);
  if (noListFamily !== null) {
    flags.push({
      reason: OUT_OF_SCOPE_NO_LIST,
      disposition:
        input.goodsConfidenceBps >= NO_LIST_HIGH_CONFIDENCE_BPS ? "reject" : "review",
      evidence: `CPV ${noListFamily.code} matches ${noListFamily.family} no-list family`,
    });
  }

  const bespokeIndicator = detectBespokeIndicator([
    input.title,
    input.description,
    input.descriptionText,
  ]);
  if (bespokeIndicator !== null) {
    flags.push({
      reason: OUT_OF_SCOPE_BESPOKE,
      disposition: "review",
      evidence: `notice text contains ${bespokeIndicator}`,
    });
  }

  return flags;
}

function detectNoListCpvFamily(
  cpvCodes: readonly string[],
): { family: string; code: string } | null {
  const families: Array<{ family: string; prefixes: readonly string[] }> = [
    { family: "pharma", prefixes: ["336"] },
    { family: "medical", prefixes: ["331", "33"] },
    { family: "chemical", prefixes: ["24"] },
    { family: "vehicles", prefixes: ["34"] },
  ];

  for (const rawCode of cpvCodes) {
    const code = rawCode.replaceAll(/[^0-9]/g, "");
    for (const family of families) {
      if (family.prefixes.some((prefix) => code.startsWith(prefix))) {
        return {
          family: family.family,
          code: rawCode,
        };
      }
    }
  }

  return null;
}

function detectBespokeIndicator(textParts: ReadonlyArray<string | null | undefined>) {
  const normalizedText = textParts
    .filter((part): part is string => part !== null && part !== undefined)
    .join(" ")
    .toLowerCase();

  const indicators: Array<[RegExp, string]> = [
    [/\bbespoke\b/, "bespoke"],
    [/\bmade\s*[- ]\s*to\s*[- ]\s*measure\b/, "made-to-measure"],
    [/\bcustom\s*[- ]\s*manufactur(?:ed|e|ing)\b/, "custom-manufactured"],
  ];

  return indicators.find(([pattern]) => pattern.test(normalizedText))?.[1] ?? null;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
