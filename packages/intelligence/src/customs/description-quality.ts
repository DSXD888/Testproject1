import type { RecallSourceFamily } from "./multi-source-recall";

export interface RawDescription {
  text: string;
  cpvCode?: string;
  lotMetadata?: Record<string, unknown>;
}

export interface ExtractedAttributes {
  headNoun: string;
  material?: string;
  principalFunction?: string;
  sector?: string;
  sterilityMedical?: "sterile" | "non_sterile" | "medical_device" | "n_a";
  powerSize?: {
    power_w?: number;
    size_mm?: number;
    capacity_l?: number;
  };
  packForm?: "retail_pack" | "bulk" | "set" | "kit" | "unknown";
}

export type DescriptionQualityVerdict =
  | { kind: "accept"; attributes: ExtractedAttributes }
  | {
      kind: "reject_generic";
      reason: string;
      missingArchetypeFacts: string[];
    }
  | { kind: "reject_service"; reason: string }
  | {
      kind: "reject_bundle_unsafe";
      reason: string;
      markAs: "mixed_manual" | "gir_3_candidate";
    }
  | { kind: "split_safe"; subDescriptions: RawDescription[] };

export interface QueryWave {
  wave: number;
  strategy: string;
  query_en: string;
  query_zh?: string;
}

export interface PlannedRecallQueryWave extends QueryWave {
  sourceFamily: RecallSourceFamily;
  familyCountKey: RecallSourceFamily;
}

export const RECALL_QUERY_SOURCE_FAMILIES: readonly RecallSourceFamily[] = [
  "hmrc_fpo_or_search_suggestion",
  "hmrc_tariff_keyword_search",
  "cpv_chapter_prior",
  "rule_based_chapter_hint",
  "atr_codebook_reuse",
];

const GENERIC_HEAD_NOUNS = new Set([
  "equipment",
  "goods",
  "items",
  "materials",
  "products",
  "supplies",
]);

const KNOWN_HEAD_NOUNS = [
  "gloves",
  "cartons",
  "boxes",
  "fasteners",
  "bolts",
  "screws",
  "wipes",
  "scissors",
  "masks",
  "batteries",
  "cables",
  "equipment",
  "goods",
  "items",
  "products",
  "supplies",
] as const;

const MATERIAL_PATTERNS: Array<[RegExp, string]> = [
  [/\bstainless\s+steel\b/, "stainless steel"],
  [/\bpaperboard\b/, "paperboard"],
  [/\bcardboard\b/, "cardboard"],
  [/\baluminium\b|\baluminum\b/, "aluminium"],
  [/\bnitrile\b/, "nitrile"],
  [/\blatex\b/, "latex"],
  [/\bvinyl\b/, "vinyl"],
  [/\brubber\b/, "rubber"],
  [/\bplastic\b/, "plastic"],
  [/\bsteel\b/, "steel"],
  [/\bcotton\b/, "cotton"],
];

export function gateDescriptionQuality(
  input: RawDescription,
): DescriptionQualityVerdict {
  const normalizedText = normalize(input.text);
  if (normalizedText.length === 0) {
    return rejectGeneric("goods", ["goods.head_noun"]);
  }

  if (isServiceDescription(normalizedText)) {
    return {
      kind: "reject_service",
      reason: "service_description_not_physical_goods",
    };
  }

  if (isRetailSet(normalizedText)) {
    return {
      kind: "reject_bundle_unsafe",
      reason: "unsafe retail set or kit requires GIR 3 manual review before recall",
      markAs: "gir_3_candidate",
    };
  }

  const splitSegments = splitBundleSegments(input.text);
  if (splitSegments.length > 1) {
    const segmentVerdicts = splitSegments.map((segment) =>
      evaluateSingleDescription({
        ...input,
        text: segment,
      }),
    );
    if (segmentVerdicts.every((verdict) => verdict.kind === "accept")) {
      return {
        kind: "split_safe",
        subDescriptions: splitSegments.map((text) => ({
          text,
          cpvCode: input.cpvCode,
          lotMetadata: input.lotMetadata,
        })),
      };
    }

    return {
      kind: "reject_bundle_unsafe",
      reason: "mixed bundle lacks independently classifiable item facts",
      markAs: "mixed_manual",
    };
  }

  const verdict = evaluateSingleDescription(input);
  if (verdict.kind === "accept") {
    return {
      kind: "accept",
      attributes: verdict.attributes,
    };
  }

  return rejectGeneric(verdict.headNoun, verdict.missingArchetypeFacts);
}

export function generateQueryWaves(attrs: ExtractedAttributes): QueryWave[] {
  const candidates: Array<Omit<QueryWave, "wave">> = [
    {
      strategy: "head_noun",
      query_en: attrs.headNoun,
    },
  ];

  if (attrs.material !== undefined) {
    candidates.push({
      strategy: "material_head_noun",
      query_en: `${attrs.material} ${attrs.headNoun}`,
    });
  }

  if (attrs.principalFunction !== undefined) {
    candidates.push({
      strategy: "function_head_noun",
      query_en: `${attrs.principalFunction} ${attrs.headNoun}`,
    });
  }

  if (
    attrs.sector !== undefined &&
    attrs.material !== undefined &&
    attrs.principalFunction !== undefined
  ) {
    candidates.push({
      strategy: "sector_material_function_head_noun",
      query_en: `${attrs.sector} ${attrs.material} ${attrs.principalFunction} ${attrs.headNoun}`,
    });
  }

  if (attrs.packForm !== undefined && attrs.material !== undefined) {
    candidates.push({
      strategy: "pack_form_material_head_noun",
      query_en: `${packFormQueryTerm(attrs.packForm)} ${attrs.material} ${attrs.headNoun}`,
    });
  }

  const uniqueCandidates = dedupeByQuery(candidates);
  return uniqueCandidates.map((candidate, index) => ({
    wave: index + 1,
    ...candidate,
  }));
}

export function planRecallQueryWaves(
  attrs: ExtractedAttributes,
  sourceFamilies: readonly RecallSourceFamily[] = RECALL_QUERY_SOURCE_FAMILIES,
): PlannedRecallQueryWave[] {
  const waves = generateQueryWaves(attrs);
  return sourceFamilies.flatMap((sourceFamily) =>
    waves.map((wave) => ({
      ...wave,
      sourceFamily,
      familyCountKey: sourceFamily,
    })),
  );
}

export function countRecallSourceFamilies(
  plannedWaves: readonly Pick<PlannedRecallQueryWave, "familyCountKey">[],
): number {
  return new Set(plannedWaves.map((wave) => wave.familyCountKey)).size;
}

type SingleDescriptionVerdict =
  | { kind: "accept"; attributes: ExtractedAttributes }
  | { kind: "reject"; headNoun: string; missingArchetypeFacts: string[] };

function evaluateSingleDescription(input: RawDescription): SingleDescriptionVerdict {
  const normalizedText = normalize(input.text);
  const headNoun = extractHeadNoun(normalizedText);
  const attributes: ExtractedAttributes = {
    headNoun,
  };

  const material = extractMaterial(normalizedText);
  if (material !== undefined) {
    attributes.material = material;
  }

  const principalFunction = extractPrincipalFunction(normalizedText, headNoun);
  if (principalFunction !== undefined) {
    attributes.principalFunction = principalFunction;
  }

  const sector = extractSector(normalizedText);
  if (sector !== undefined) {
    attributes.sector = sector;
  }

  const sterilityMedical = extractSterilityMedical(normalizedText);
  if (sterilityMedical !== undefined) {
    attributes.sterilityMedical = sterilityMedical;
  }

  const powerSize = extractPowerSize(normalizedText);
  if (Object.keys(powerSize).length > 0) {
    attributes.powerSize = powerSize;
  }

  const packForm = extractPackForm(normalizedText);
  if (packForm !== undefined) {
    attributes.packForm = packForm;
  }

  const missingArchetypeFacts = missingFactsFor(headNoun, attributes);
  if (GENERIC_HEAD_NOUNS.has(headNoun) || missingArchetypeFacts.length > 0) {
    return {
      kind: "reject",
      headNoun,
      missingArchetypeFacts,
    };
  }

  return {
    kind: "accept",
    attributes,
  };
}

function rejectGeneric(
  headNoun: string,
  missingArchetypeFacts: string[],
): DescriptionQualityVerdict {
  return {
    kind: "reject_generic",
    reason: "missing_required_archetype_facts_before_recall",
    missingArchetypeFacts,
  };
}

function missingFactsFor(
  headNoun: string,
  attrs: ExtractedAttributes,
): string[] {
  const facts: string[] = [];
  const requiredFacts = requiredFactsFor(headNoun);
  for (const fact of requiredFacts) {
    if (fact === "material" && attrs.material === undefined) {
      facts.push(`${headNoun}.material`);
    }
    if (fact === "principal_function" && attrs.principalFunction === undefined) {
      facts.push(`${headNoun}.principal_function`);
    }
    if (fact === "sector" && attrs.sector === undefined) {
      facts.push(`${headNoun}.sector`);
    }
    if (fact === "sterility_medical" && attrs.sterilityMedical === undefined) {
      facts.push(`${headNoun}.sterility_medical`);
    }
    if (fact === "pack_form" && attrs.packForm === undefined) {
      facts.push(`${headNoun}.pack_form`);
    }
    if (fact === "power_size" && attrs.powerSize === undefined) {
      facts.push(`${headNoun}.power_size`);
    }
  }
  return facts;
}

function requiredFactsFor(headNoun: string): string[] {
  if (headNoun === "gloves") {
    return [
      "material",
      "principal_function",
      "sector",
      "sterility_medical",
      "pack_form",
    ];
  }
  if (headNoun === "cartons" || headNoun === "boxes") {
    return ["material", "principal_function", "pack_form"];
  }
  if (headNoun === "fasteners" || headNoun === "bolts" || headNoun === "screws") {
    return ["material", "principal_function"];
  }
  if (GENERIC_HEAD_NOUNS.has(headNoun)) {
    return ["material", "principal_function", "sector"];
  }
  return ["material", "principal_function"];
}

function splitBundleSegments(text: string): string[] {
  const normalizedText = text
    .replace(/\s*[;+]\s*/g, " and ")
    .replace(/\s*,\s*/g, " and ");
  const segments = normalizedText
    .split(/\s+and\s+/i)
    .map((segment) => segment.trim())
    .filter(Boolean);

  const distinctHeadNouns = new Set(
    segments.map((segment) => extractHeadNoun(normalize(segment))),
  );
  return segments.length > 1 && distinctHeadNouns.size > 1 ? segments : [text.trim()];
}

function isServiceDescription(text: string): boolean {
  return /\b(consulting|installation|maintenance|managed|repair|services?|support|training)\b/.test(
    text,
  );
}

function isRetailSet(text: string): boolean {
  return /\b(first\s+aid\s+kit|kit|set)\b/.test(text);
}

function extractHeadNoun(text: string): string {
  for (const noun of KNOWN_HEAD_NOUNS) {
    if (new RegExp(`\\b${escapeRegExp(noun)}\\b`).test(text)) {
      return noun;
    }
  }
  const tokens = text.split(/\s+/).filter(Boolean);
  return tokens.at(-1) ?? "goods";
}

function extractMaterial(text: string): string | undefined {
  return MATERIAL_PATTERNS.find(([pattern]) => pattern.test(text))?.[1];
}

function extractPrincipalFunction(
  text: string,
  headNoun: string,
): string | undefined {
  if (/\b(examination|exam)\b/.test(text) && headNoun === "gloves") {
    return "examination protection";
  }
  if (/\bsurgical\b/.test(text) && headNoun === "gloves") {
    return "surgical protection";
  }
  if (/\b(packaging|packing|shipping)\b/.test(text)) {
    return "packaging";
  }
  if (/\b(fastening|fixing|anchor|anchoring)\b/.test(text)) {
    return "fastening";
  }
  if (/\bcleaning|wipe|wiping\b/.test(text)) {
    return "cleaning";
  }
  return undefined;
}

function extractSector(text: string): string | undefined {
  if (/\b(medical|clinical|surgical|examination|hospital|healthcare)\b/.test(text)) {
    return "medical";
  }
  if (/\b(food|catering|kitchen)\b/.test(text)) {
    return "food";
  }
  if (/\b(packaging|carton|cartons|boxes)\b/.test(text)) {
    return "packaging";
  }
  if (/\b(electrical|electronics|battery|batteries|cable|cables)\b/.test(text)) {
    return "electrical";
  }
  return undefined;
}

function extractSterilityMedical(
  text: string,
): ExtractedAttributes["sterilityMedical"] | undefined {
  if (/\bnon[-\s]?sterile\b/.test(text)) {
    return "non_sterile";
  }
  if (/\bsterile\b/.test(text)) {
    return "sterile";
  }
  if (/\bmedical\s+device\b/.test(text)) {
    return "medical_device";
  }
  if (/\bmedical|clinical|surgical|examination|hospital|healthcare\b/.test(text)) {
    return "medical_device";
  }
  return undefined;
}

function extractPowerSize(text: string): NonNullable<ExtractedAttributes["powerSize"]> {
  const powerSize: NonNullable<ExtractedAttributes["powerSize"]> = {};
  const powerMatch = text.match(/\b(?<value>\d+(?:\.\d+)?)\s*w\b/);
  const sizeMatch = text.match(/\b(?<value>\d+(?:\.\d+)?)\s*mm\b/);
  const capacityMatch = text.match(/\b(?<value>\d+(?:\.\d+)?)\s*l\b/);

  if (powerMatch?.groups?.value !== undefined) {
    powerSize.power_w = Number(powerMatch.groups.value);
  }
  if (sizeMatch?.groups?.value !== undefined) {
    powerSize.size_mm = Number(sizeMatch.groups.value);
  }
  if (capacityMatch?.groups?.value !== undefined) {
    powerSize.capacity_l = Number(capacityMatch.groups.value);
  }

  return powerSize;
}

function extractPackForm(text: string): ExtractedAttributes["packForm"] | undefined {
  if (/\bbulk\b/.test(text)) {
    return "bulk";
  }
  if (/\bretail\s+pack|consumer\s+pack\b/.test(text)) {
    return "retail_pack";
  }
  if (/\bkit\b/.test(text)) {
    return "kit";
  }
  if (/\bset\b/.test(text)) {
    return "set";
  }
  if (/\bpack\b/.test(text)) {
    return "retail_pack";
  }
  return undefined;
}

function packFormQueryTerm(packForm: NonNullable<ExtractedAttributes["packForm"]>): string {
  if (packForm === "retail_pack") {
    return "retail pack";
  }
  if (packForm === "unknown") {
    return "";
  }
  return packForm;
}

function dedupeByQuery(
  candidates: Array<Omit<QueryWave, "wave">>,
): Array<Omit<QueryWave, "wave">> {
  const seen = new Set<string>();
  const unique: Array<Omit<QueryWave, "wave">> = [];
  for (const candidate of candidates) {
    const query = normalizeQuery(candidate.query_en);
    if (query.length === 0 || seen.has(query)) {
      continue;
    }
    seen.add(query);
    unique.push({
      ...candidate,
      query_en: query,
    });
  }
  return unique;
}

function normalize(value: string): string {
  return value.toLowerCase().replace(/\s+/g, " ").trim();
}

function normalizeQuery(value: string): string {
  return normalize(value.replace(/\s+/g, " "));
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
