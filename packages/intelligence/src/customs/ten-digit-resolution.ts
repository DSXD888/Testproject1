import { createHash } from "node:crypto";

import type { ExtractedAttributes } from "./description-quality";
import type {
  MultiSourceRecallResult,
  SourceFamily,
  SourceFamilyResult,
} from "./multi-source-recall";

export const CLASSIFICATION_AMBIGUOUS = "CLASSIFICATION_AMBIGUOUS" as const;
export const TARIFF_BRANCH_DIVERGENCE = "TARIFF_BRANCH_DIVERGENCE" as const;
export const CUSTOMS_BRANCH_DIVERGENCE_BPS = 1_500;

export type TariffNodeLevel = "chapter" | "heading" | "subheading" | "cn8" | "uk10";

export interface TariffMeasure {
  id: string;
  kind:
    | "third_country_duty"
    | "import_vat"
    | "anti_dumping"
    | "suspension"
    | "quota"
    | "documentary";
  description: string;
  dutyExpression?: string;
}

export interface AdditionalCodeRequirement {
  codeType: string;
  description: string;
  requiredWhen: string;
}

export interface TariffFreshness {
  structureCheckedAt: string;
  measuresFreshUntil: string;
}

export interface TariffNode {
  code: string;
  parent: string | null;
  level: TariffNodeLevel;
  description: string;
  measures: TariffMeasure[];
  supplementaryUnitRequirements: string[];
  additionalCodeRequirements: AdditionalCodeRequirement[];
  scopeNotes: string[];
  exclusions: string[];
  freshness: TariffFreshness;
}

export interface ScoredTariffNode {
  node: TariffNode;
  scoreBps: number;
  compatibilityReasons: string[];
  rejectionReasons: string[];
}

export interface TariffGraph {
  getNode(code: string): TariffNode | null;
  descendants(code: string): TariffNode[];
  shortlistNonDominated(
    family: string,
    attrs: ExtractedAttributes,
    count: number,
  ): ScoredTariffNode[];
}

export interface ExpandedTariffCandidate extends ScoredTariffNode {
  familyCode: string;
  initialRecallRank: number;
  rankOrdinal: number;
  sourceFamilies: SourceFamily[];
  retrievalScoreBps: number;
}

export interface TariffFamilySummary {
  familyCode: string;
  initialRecallRank: number;
  expandedDescendantCount: number;
  shortlistedCount: number;
  mostSupportedBranch: string | null;
}

export interface TariffGraphExpansionResult {
  expandedCandidates: ExpandedTariffCandidate[];
  shortlistedCandidates: ExpandedTariffCandidate[];
  selectedCandidate: ExpandedTariffCandidate | null;
  verificationQueue: string[];
  familySummaries: TariffFamilySummary[];
}

export interface UkTenDigitCandidate {
  uk10Code: string;
  parentHs6: string;
  rank: number;
  scoreBps: number;
  dutyOutcomePence?: number;
  dutyType?: "ad_valorem" | "specific" | "compound" | "unknown";
  hasTradeRemedy?: boolean;
  slotCompatibility: {
    descriptionMatchBps: number;
    materialMatchBps: number;
    unitCompatibility: "ok" | "missing" | "incompatible";
    exclusionsTriggered: string[];
    scopeNoteFlags: string[];
  };
  candidateRecallAgreement: SourceFamilyResult[];
}

export type TenDigitCandidate = UkTenDigitCandidate;

export type DivergenceKind =
  | "remedy_no_remedy"
  | "duty_type"
  | "material_spread";

export type DivergenceErrorCode =
  | typeof CLASSIFICATION_AMBIGUOUS
  | typeof TARIFF_BRANCH_DIVERGENCE;

export interface DivergenceCheckResult {
  divergent: boolean;
  divergenceKind?: DivergenceKind;
  customsCostSpreadBps: number;
  emittedCodes: DivergenceErrorCode[];
  selectedBranchAuto: TenDigitCandidate | null;
  worstCaseDutyPence: number;
  bestCaseDutyPence: number;
}

export interface FamilyClusterSummary {
  familyLabel: string;
  count: number;
  conservativeDutyOutcome: { dutyPctBps: number; rationale: string };
  mostSupportedBranch: UkTenDigitCandidate;
  blockers: string[];
  missingFacts: string[];
}

export interface TenDigitLatticeSeparation {
  recallFamilyCodes: string[];
  classificationCandidateCodes: string[];
  shortlistedBeforeCommercialScenarios: true;
  commercialScenarioInputsUsed: string[];
}

export interface TenDigitResolutionResult {
  shortlisted: UkTenDigitCandidate[];
  perFamilyClusters: FamilyClusterSummary[];
  greedyDescentDetected: false;
  candidateCommodityCodesJson: string[];
  selectedUk10: string | null;
  rejectionReasonForTopRanked: string | null;
  latticeSeparation: TenDigitLatticeSeparation;
}

export interface TenDigitOperatorSurfaceVm {
  kind: "family_clusters";
  clusters: FamilyClusterSummary[];
}

export interface TariffBranchVerification {
  code: string;
  verifiedAt: string;
  measuresFreshUntil: string;
  measures: TariffMeasure[];
}

export interface ProductCommodityCodeRow {
  id: string;
  productCandidateId: string;
  commodityCode: string;
  digits: 6 | 8 | 10;
  source: SourceFamily;
  scoreBps: number;
  rankOrdinal: number;
  description: string;
  evidenceJson: string;
  ownerId: string;
  rowVersion: number;
  lastLocalSeq: number;
  stageRunId: string;
}

export interface ProductCommodityCodeOutboxEntry {
  localSeq: number;
  entityKind: "product_commodity_codes";
  entityId: string;
  op: "upsert";
  rowVersion: number;
  stageRunId: string;
  commitGroupId: string;
  payloadJson: string;
  contentHash: string;
}

export interface RecallLatticePersistenceCandidate {
  code: string;
  rank: number;
  sourceFamily: SourceFamily;
  retrievalScoreBps: number | null;
  agreementState: "agreed" | "diverged" | "unavailable";
  description?: string;
}

export interface RecallLatticePersistenceTransaction {
  replaceProductCommodityCodes(productCandidateId: string): void | Promise<void>;
  nextLocalSeq(): number | Promise<number>;
  insertProductCommodityCode(row: ProductCommodityCodeRow): void | Promise<void>;
  appendLocalOutbox(entry: ProductCommodityCodeOutboxEntry): void | Promise<void>;
}

export interface RecallLatticePersistenceStore {
  transaction<T>(
    commitGroupId: string,
    fn: (tx: RecallLatticePersistenceTransaction) => T | Promise<T>,
  ): Promise<T>;
}

export class InMemoryTariffGraph implements TariffGraph {
  private readonly nodes: Map<string, TariffNode>;

  constructor(nodes: TariffNode[]) {
    this.nodes = new Map(nodes.map((node) => [node.code, node]));
  }

  getNode(code: string): TariffNode | null {
    return this.nodes.get(code) ?? null;
  }

  descendants(code: string): TariffNode[] {
    const start = this.nodes.get(code);
    if (start === undefined) {
      return [];
    }

    const descendants: TariffNode[] = [];
    const pending = [start.code];
    while (pending.length > 0) {
      const parent = pending.shift();
      if (parent === undefined) {
        continue;
      }

      for (const child of this.nodes.values()) {
        if (child.parent !== parent) {
          continue;
        }

        if (child.level === "uk10") {
          descendants.push(child);
        }
        pending.push(child.code);
      }
    }

    if (start.level === "uk10") {
      descendants.push(start);
    }

    return descendants.sort((left, right) => left.code.localeCompare(right.code));
  }

  shortlistNonDominated(
    family: string,
    attrs: ExtractedAttributes,
    count: number,
  ): ScoredTariffNode[] {
    return this.descendants(family)
      .map((node) => scoreTariffNode(node, attrs, 0))
      .sort(compareScoredTariffNodes)
      .slice(0, count);
  }
}

export class InMemoryRecallLatticeStore implements RecallLatticePersistenceStore {
  rows: ProductCommodityCodeRow[];
  outbox: ProductCommodityCodeOutboxEntry[];
  transactions: Array<{ commitGroupId: string; insertedRowIds: string[] }>;
  private localSeq: number;

  constructor(opts?: {
    rows?: ProductCommodityCodeRow[];
    outbox?: ProductCommodityCodeOutboxEntry[];
    localSeq?: number;
  }) {
    this.rows = opts?.rows ?? [];
    this.outbox = opts?.outbox ?? [];
    this.transactions = [];
    this.localSeq = opts?.localSeq ?? 0;
  }

  async transaction<T>(
    commitGroupId: string,
    fn: (tx: RecallLatticePersistenceTransaction) => T | Promise<T>,
  ): Promise<T> {
    const rowsBefore = [...this.rows];
    const outboxBefore = [...this.outbox];
    const localSeqBefore = this.localSeq;
    const insertedRowIds: string[] = [];

    const tx: RecallLatticePersistenceTransaction = {
      replaceProductCommodityCodes: (productCandidateId) => {
        this.rows = this.rows.filter(
          (row) => row.productCandidateId !== productCandidateId,
        );
      },
      nextLocalSeq: () => {
        this.localSeq += 1;
        return this.localSeq;
      },
      insertProductCommodityCode: (row) => {
        this.rows.push(row);
        insertedRowIds.push(row.id);
      },
      appendLocalOutbox: (entry) => {
        this.outbox.push(entry);
      },
    };

    try {
      const result = await fn(tx);
      this.transactions.push({ commitGroupId, insertedRowIds });
      return result;
    } catch (error) {
      this.rows = rowsBefore;
      this.outbox = outboxBefore;
      this.localSeq = localSeqBefore;
      throw error;
    }
  }
}

export class TenDigitResolution {
  constructor(
    private readonly graph: TariffGraph,
    private readonly opts: { shortlistPerFamily?: number } = {},
  ) {}

  resolve(
    attributes: ExtractedAttributes,
    recall: MultiSourceRecallResult,
  ): TenDigitResolutionResult {
    const recallWithAttributes = { ...recall, attributes };
    const expansion = expandRecallLatticeToTariffGraph(this.graph, recallWithAttributes, {
      shortlistPerFamily: this.opts.shortlistPerFamily ?? 3,
    });
    const shortlisted = expansion.shortlistedCandidates.map((candidate) =>
      toUkTenDigitCandidate(candidate, recallWithAttributes, attributes),
    );
    const selectedUk10 = expansion.selectedCandidate?.node.code ?? null;

    return {
      shortlisted,
      perFamilyClusters: buildFamilyClusters(
        this.graph,
        expansion,
        recallWithAttributes,
        attributes,
      ),
      greedyDescentDetected: false,
      candidateCommodityCodesJson: uniqueSorted(
        shortlisted.map((candidate) => candidate.uk10Code),
      ),
      selectedUk10,
      rejectionReasonForTopRanked: topRankedRejectionReason(
        expansion,
        attributes,
        selectedUk10,
      ),
      latticeSeparation: {
        recallFamilyCodes: expansion.familySummaries.map((family) => family.familyCode),
        classificationCandidateCodes: uniqueSorted(
          expansion.expandedCandidates.map((candidate) => candidate.node.code),
        ),
        shortlistedBeforeCommercialScenarios: true,
        commercialScenarioInputsUsed: [],
      },
    };
  }
}

export function createTenDigitOperatorSurface(
  result: TenDigitResolutionResult,
  opts: { mode?: "family_clusters" | "flat_list" } = {},
): TenDigitOperatorSurfaceVm {
  if (opts.mode === "flat_list") {
    throw new Error("flat ungrouped UK10 lists are not an acceptable operator surface");
  }

  return {
    kind: "family_clusters",
    clusters: result.perFamilyClusters,
  };
}

export function detectBranchDivergence(
  branches: readonly TenDigitCandidate[],
): DivergenceCheckResult {
  if (branches.length === 0) {
    return {
      divergent: false,
      customsCostSpreadBps: 0,
      emittedCodes: [],
      selectedBranchAuto: null,
      worstCaseDutyPence: 0,
      bestCaseDutyPence: 0,
    };
  }

  const sortedBranches = [...branches].sort(compareTenDigitCandidates);
  const dutyOutcomes = sortedBranches.map((branch) => branch.dutyOutcomePence ?? 0);
  const worstCaseDutyPence = Math.max(...dutyOutcomes);
  const bestCaseDutyPence = Math.min(...dutyOutcomes);
  const customsCostSpreadBps = dutySpreadBps(
    bestCaseDutyPence,
    worstCaseDutyPence,
  );
  const hasRemedyBranch = sortedBranches.some(
    (branch) => branch.hasTradeRemedy === true,
  );
  const hasNoRemedyBranch = sortedBranches.some(
    (branch) => branch.hasTradeRemedy === false,
  );
  const dutyTypeCount = new Set(
    sortedBranches.flatMap((branch) =>
      branch.dutyType === undefined ? [] : [branch.dutyType],
    ),
  ).size;

  const divergenceKind = branchDivergenceKind({
    hasRemedyBranch,
    hasNoRemedyBranch,
    dutyTypeCount,
    customsCostSpreadBps,
  });
  const divergent = divergenceKind !== undefined;

  return {
    divergent,
    ...(divergenceKind === undefined ? {} : { divergenceKind }),
    customsCostSpreadBps,
    emittedCodes: divergent
      ? divergenceErrorCodes(sortedBranches)
      : [],
    selectedBranchAuto: divergent ? null : sortedBranches[0] ?? null,
    worstCaseDutyPence,
    bestCaseDutyPence,
  };
}

export function expandRecallLatticeToTariffGraph(
  graph: TariffGraph,
  recallResult: MultiSourceRecallResult,
  opts?: { shortlistPerFamily?: number },
): TariffGraphExpansionResult {
  const shortlistPerFamily = opts?.shortlistPerFamily ?? 3;
  const familyInputs = recallResult.candidateRecallAgreement.map(
    (agreement, index) => {
      const agreedSources = agreement.agreement.filter(
        (sourceResult) => sourceResult.state === "agreed",
      );
      return {
        familyCode: agreement.candidateCode,
        initialRecallRank: index + 1,
        sourceFamilies: agreedSources.map((sourceResult) => sourceResult.sourceFamily),
        retrievalScoreBps: Math.max(
          0,
          ...agreedSources.map((sourceResult) => sourceResult.scoreBps ?? 0),
        ),
      };
    },
  );

  const expandedCandidates = familyInputs.flatMap((familyInput) =>
    graph.descendants(familyInput.familyCode).map((node) => ({
      ...scoreTariffNode(node, recallResult.attributes, familyInput.retrievalScoreBps),
      familyCode: familyInput.familyCode,
      initialRecallRank: familyInput.initialRecallRank,
      rankOrdinal: familyInput.initialRecallRank,
      sourceFamilies: familyInput.sourceFamilies,
      retrievalScoreBps: familyInput.retrievalScoreBps,
    })),
  );

  const shortlistedCandidates = familyInputs.flatMap((familyInput) =>
    expandedCandidates
      .filter((candidate) => candidate.familyCode === familyInput.familyCode)
      .sort(compareExpandedTariffCandidates)
      .slice(0, shortlistPerFamily)
      .map((candidate, index) => ({
        ...candidate,
        rankOrdinal: index + 1,
      })),
  );

  const selectedCandidate = [...shortlistedCandidates].sort(
    compareExpandedTariffCandidates,
  )[0] ?? null;

  return {
    expandedCandidates,
    shortlistedCandidates,
    selectedCandidate,
    verificationQueue: shortlistedCandidates.map((candidate) => candidate.node.code),
    familySummaries: familyInputs.map((familyInput) => {
      const familyShortlist = shortlistedCandidates.filter(
        (candidate) => candidate.familyCode === familyInput.familyCode,
      );
      return {
        familyCode: familyInput.familyCode,
        initialRecallRank: familyInput.initialRecallRank,
        expandedDescendantCount: expandedCandidates.filter(
          (candidate) => candidate.familyCode === familyInput.familyCode,
        ).length,
        shortlistedCount: familyShortlist.length,
        mostSupportedBranch: familyShortlist[0]?.node.code ?? null,
      };
    }),
  };
}

export async function verifyShortlistedTariffBranches(
  expansion: TariffGraphExpansionResult,
  verifier: (node: TariffNode) => TariffBranchVerification | Promise<TariffBranchVerification>,
): Promise<TariffBranchVerification[]> {
  const uniqueShortlist = uniqueBy(
    expansion.shortlistedCandidates.map((candidate) => candidate.node),
    (node) => node.code,
  );

  const verifications: TariffBranchVerification[] = [];
  for (const node of uniqueShortlist) {
    verifications.push(await verifier(node));
  }

  return verifications;
}

export function recallLatticeRowsFromExpansion(
  expansion: TariffGraphExpansionResult,
): RecallLatticePersistenceCandidate[] {
  return expansion.shortlistedCandidates.flatMap(
    (candidate): RecallLatticePersistenceCandidate[] => {
      if (candidate.sourceFamilies.length === 0) {
        return [
          {
            code: candidate.node.code,
            rank: candidate.rankOrdinal,
            sourceFamily: "hmrc_tariff_keyword_search",
            retrievalScoreBps: candidate.retrievalScoreBps,
            agreementState: "unavailable",
            description: candidate.node.description,
          },
        ];
      }

      return candidate.sourceFamilies.map((sourceFamily) => ({
        code: candidate.node.code,
        rank: candidate.rankOrdinal,
        sourceFamily,
        retrievalScoreBps: candidate.retrievalScoreBps,
        agreementState: "agreed",
        description: candidate.node.description,
      }));
    },
  );
}

export async function persistRecallLattice(
  db: RecallLatticePersistenceStore,
  opts: {
    productCandidateId: string;
    stageRunId: string;
    recallResult: MultiSourceRecallResult;
    expandedCandidates: RecallLatticePersistenceCandidate[];
    ownerId?: string;
    commitGroupId?: string;
  },
): Promise<ProductCommodityCodeRow[]> {
  const ownerId = opts.ownerId ?? "local-daemon";
  const commitGroupId =
    opts.commitGroupId ??
    `${opts.stageRunId}:product_commodity_codes:${opts.productCandidateId}`;
  const candidates = dedupePersistenceCandidates(opts.expandedCandidates);
  const writtenRows: ProductCommodityCodeRow[] = [];

  await db.transaction(commitGroupId, async (tx) => {
    await tx.replaceProductCommodityCodes(opts.productCandidateId);

    for (const candidate of candidates) {
      const localSeq = await tx.nextLocalSeq();
      const row: ProductCommodityCodeRow = {
        id: productCommodityCodeId(opts.productCandidateId, candidate),
        productCandidateId: opts.productCandidateId,
        commodityCode: candidate.code,
        digits: commodityDigits(candidate.code),
        source: candidate.sourceFamily,
        scoreBps: candidate.retrievalScoreBps ?? 0,
        rankOrdinal: candidate.rank,
        description: candidate.description ?? "",
        evidenceJson: stableJson({
          agreementState: candidate.agreementState,
          sourceFamily: candidate.sourceFamily,
          candidateRecallAgreement: opts.recallResult.candidateRecallAgreement.map(
            (agreement) => ({
              candidateCode: agreement.candidateCode,
              states: agreement.agreement.map((sourceResult) => ({
                sourceFamily: sourceResult.sourceFamily,
                state: sourceResult.state,
              })),
            }),
          ),
        }),
        ownerId,
        rowVersion: 1,
        lastLocalSeq: localSeq,
        stageRunId: opts.stageRunId,
      };
      const payloadJson = stableJson(row);
      const outboxEntry: ProductCommodityCodeOutboxEntry = {
        localSeq,
        entityKind: "product_commodity_codes",
        entityId: row.id,
        op: "upsert",
        rowVersion: row.rowVersion,
        stageRunId: opts.stageRunId,
        commitGroupId,
        payloadJson,
        contentHash: sha256(payloadJson),
      };

      await tx.insertProductCommodityCode(row);
      await tx.appendLocalOutbox(outboxEntry);
      writtenRows.push(row);
    }
  });

  return writtenRows;
}

const SOURCE_FAMILY_ORDER: SourceFamily[] = [
  "hmrc_fpo_or_search_suggestion",
  "cpv_chapter_prior",
  "rule_based_chapter_hint",
  "hmrc_tariff_keyword_search",
  "atr_codebook_reuse",
];

function scoreTariffNode(
  node: TariffNode,
  attrs: ExtractedAttributes,
  retrievalScoreBps: number,
): ScoredTariffNode {
  const haystack = tokenize(
    [
      node.description,
      ...node.scopeNotes,
      ...node.exclusions,
      ...node.measures.map((measure) => measure.description),
    ].join(" "),
  );
  const compatibilityReasons: string[] = [];
  const rejectionReasons: string[] = [];
  let score = Math.round(retrievalScoreBps * 0.45);

  if (matchesAny(haystack, nounTerms(attrs.headNoun))) {
    score += 2_000;
    compatibilityReasons.push("head_noun_compatible");
  }

  if (attrs.material !== undefined) {
    const classificationText = tokenize(
      [
        node.description,
        ...node.scopeNotes,
        ...node.measures.map((measure) => measure.description),
      ].join(" "),
    );
    if (matchesAny(classificationText, tokenize(attrs.material))) {
      score += 2_000;
      compatibilityReasons.push("material_compatible");
    } else if (materialMismatch(node, attrs.material)) {
      score -= 3_500;
      rejectionReasons.push("material_mismatch");
    }
  }

  if (
    attrs.principalFunction !== undefined &&
    matchesAny(haystack, tokenize(attrs.principalFunction))
  ) {
    score += 1_000;
    compatibilityReasons.push("function_compatible");
  }

  if (node.supplementaryUnitRequirements.length > 0) {
    compatibilityReasons.push("supplementary_unit_requirements_cached");
  }

  if (node.additionalCodeRequirements.length > 0) {
    compatibilityReasons.push("additional_code_requirements_cached");
  }

  return {
    node,
    scoreBps: clampBps(score),
    compatibilityReasons,
    rejectionReasons,
  };
}

function toUkTenDigitCandidate(
  candidate: ExpandedTariffCandidate,
  recall: MultiSourceRecallResult,
  attributes: ExtractedAttributes,
): UkTenDigitCandidate {
  return {
    uk10Code: candidate.node.code,
    parentHs6: candidate.familyCode,
    rank: candidate.rankOrdinal,
    scoreBps: candidate.scoreBps,
    dutyType: dutyTypeForMeasures(candidate.node.measures),
    hasTradeRemedy: candidate.node.measures.some(isTradeRemedyMeasure),
    slotCompatibility: scoreSlotCompatibility(candidate.node, attributes),
    candidateRecallAgreement: agreementForFamily(recall, candidate.familyCode),
  };
}

function buildFamilyClusters(
  graph: TariffGraph,
  expansion: TariffGraphExpansionResult,
  recall: MultiSourceRecallResult,
  attributes: ExtractedAttributes,
): FamilyClusterSummary[] {
  return expansion.familySummaries.flatMap((familySummary) => {
    const familyCandidates = expansion.expandedCandidates.filter(
      (candidate) => candidate.familyCode === familySummary.familyCode,
    );
    const familyShortlist = expansion.shortlistedCandidates.filter(
      (candidate) => candidate.familyCode === familySummary.familyCode,
    );
    const mostSupported = familyShortlist[0];
    if (mostSupported === undefined) {
      return [];
    }

    const branch = toUkTenDigitCandidate(mostSupported, recall, attributes);
    return [
      {
        familyLabel: familyLabel(graph, familySummary.familyCode),
        count: familySummary.expandedDescendantCount,
        conservativeDutyOutcome: conservativeDutyOutcome(familyCandidates),
        mostSupportedBranch: branch,
        blockers: uniqueSorted(
          familyShortlist.flatMap((candidate) => candidate.rejectionReasons),
        ),
        missingFacts: missingFactsForFamily(familyShortlist),
      },
    ];
  });
}

function scoreSlotCompatibility(
  node: TariffNode,
  attributes: ExtractedAttributes,
): UkTenDigitCandidate["slotCompatibility"] {
  const descriptionTokens = tokenize(
    [node.description, ...node.scopeNotes, ...node.measures.map((measure) => measure.description)].join(
      " ",
    ),
  );
  const descriptionMatchBps = matchesAny(descriptionTokens, nounTerms(attributes.headNoun))
    ? 10_000
    : 0;
  const materialMatchBps =
    attributes.material === undefined
      ? 5_000
      : matchesAny(descriptionTokens, tokenize(attributes.material))
        ? 10_000
        : materialMismatch(node, attributes.material)
          ? 0
          : 2_500;
  const relevantTokens = tokenize(
    [
      attributes.headNoun,
      attributes.material ?? "",
      attributes.principalFunction ?? "",
    ].join(" "),
  );

  return {
    descriptionMatchBps,
    materialMatchBps,
    unitCompatibility:
      node.supplementaryUnitRequirements.length > 0 ? "missing" : "ok",
    exclusionsTriggered: node.exclusions.filter((exclusion) =>
      matchesAny(tokenize(exclusion), relevantTokens),
    ),
    scopeNoteFlags: node.scopeNotes.filter((note) =>
      matchesAny(tokenize(note), relevantTokens),
    ),
  };
}

function agreementForFamily(
  recall: MultiSourceRecallResult,
  familyCode: string,
): SourceFamilyResult[] {
  const sourceByFamily = new Map(
    recall.candidateRecallAgreement
      .find((agreement) => agreement.candidateCode === familyCode)
      ?.agreement.map((result) => [result.sourceFamily, result]),
  );

  return SOURCE_FAMILY_ORDER.map(
    (sourceFamily) =>
      sourceByFamily.get(sourceFamily) ?? {
        sourceFamily,
        state: "unavailable",
        candidates: [],
        scoreBps: null,
        failureMode: "none",
      },
  );
}

function familyLabel(graph: TariffGraph, familyCode: string): string {
  const familyNode = graph.getNode(familyCode);
  return `${familyCode} - ${familyNode?.description ?? "Unknown HMRC tariff family"}`;
}

function conservativeDutyOutcome(
  familyCandidates: ExpandedTariffCandidate[],
): { dutyPctBps: number; rationale: string } {
  const rates = familyCandidates.flatMap((candidate) =>
    candidate.node.measures.flatMap((measure) =>
      parseDutyExpressionBps(measure.dutyExpression),
    ),
  );
  const dutyPctBps = Math.max(0, ...rates);

  return {
    dutyPctBps,
    rationale:
      dutyPctBps > 0
        ? "highest rate-bearing measure in expanded HMRC family"
        : "no rate-bearing measure found in expanded HMRC family",
  };
}

function parseDutyExpressionBps(dutyExpression?: string): number[] {
  if (dutyExpression === undefined) {
    return [];
  }
  const match = dutyExpression.match(/(\d+(?:\.\d+)?)\s*%/);
  if (match === null) {
    return [];
  }
  return [Math.round(Number(match[1]) * 100)];
}

function missingFactsForFamily(
  candidates: ExpandedTariffCandidate[],
): string[] {
  return uniqueSorted(
    candidates.flatMap((candidate) => [
      ...candidate.node.supplementaryUnitRequirements.map(
        (unit) => `supplementary_unit:${unit}`,
      ),
      ...candidate.node.additionalCodeRequirements.map(
        (requirement) => `additional_code:${requirement.codeType}`,
      ),
    ]),
  );
}

function topRankedRejectionReason(
  expansion: TariffGraphExpansionResult,
  attributes: ExtractedAttributes,
  selectedUk10: string | null,
): string | null {
  const topFamily = [...expansion.familySummaries].sort(
    (left, right) => left.initialRecallRank - right.initialRecallRank,
  )[0];
  if (topFamily === undefined || selectedUk10 === null) {
    return null;
  }
  const selected = expansion.shortlistedCandidates.find(
    (candidate) => candidate.node.code === selectedUk10,
  );
  if (selected?.familyCode === topFamily.familyCode) {
    return null;
  }

  const rejected = expansion.expandedCandidates.find(
    (candidate) =>
      candidate.familyCode === topFamily.familyCode &&
      candidate.rejectionReasons.includes("material_mismatch"),
  );
  if (rejected === undefined || attributes.material === undefined) {
    return null;
  }

  return `material mismatch on ${topFamily.familyCode} (${nodeMaterialLabel(
    rejected.node,
  )} vs ${attributes.material})`;
}

function nodeMaterialLabel(node: TariffNode): string {
  const description = node.description.toLowerCase();
  if (description.includes("rigid plastic")) {
    return "rigid plastic";
  }
  if (description.includes("plastic")) {
    return "plastic";
  }
  if (description.includes("paperboard")) {
    return "paperboard";
  }
  if (description.includes("steel")) {
    return "steel";
  }
  return "unknown material";
}

function compareExpandedTariffCandidates(
  left: ExpandedTariffCandidate,
  right: ExpandedTariffCandidate,
): number {
  return compareScoredTariffNodes(left, right);
}

function compareScoredTariffNodes(
  left: ScoredTariffNode,
  right: ScoredTariffNode,
): number {
  return right.scoreBps - left.scoreBps || left.node.code.localeCompare(right.node.code);
}

function compareTenDigitCandidates(
  left: TenDigitCandidate,
  right: TenDigitCandidate,
): number {
  return (
    left.rank - right.rank ||
    right.scoreBps - left.scoreBps ||
    left.uk10Code.localeCompare(right.uk10Code)
  );
}

function dutySpreadBps(bestCaseDutyPence: number, worstCaseDutyPence: number): number {
  if (bestCaseDutyPence <= 0) {
    return worstCaseDutyPence <= 0 ? 0 : Number.MAX_SAFE_INTEGER;
  }

  return Math.round(
    ((worstCaseDutyPence - bestCaseDutyPence) * 10_000) / bestCaseDutyPence,
  );
}

function branchDivergenceKind(input: {
  hasRemedyBranch: boolean;
  hasNoRemedyBranch: boolean;
  dutyTypeCount: number;
  customsCostSpreadBps: number;
}): DivergenceKind | undefined {
  if (input.hasRemedyBranch && input.hasNoRemedyBranch) {
    return "remedy_no_remedy";
  }
  if (input.dutyTypeCount > 1) {
    return "duty_type";
  }
  if (input.customsCostSpreadBps >= CUSTOMS_BRANCH_DIVERGENCE_BPS) {
    return "material_spread";
  }
  return undefined;
}

function divergenceErrorCodes(
  branches: readonly TenDigitCandidate[],
): DivergenceErrorCode[] {
  const codes: DivergenceErrorCode[] = [TARIFF_BRANCH_DIVERGENCE];
  const familyCount = new Set(branches.map((branch) => branch.parentHs6)).size;
  if (familyCount > 1) {
    codes.push(CLASSIFICATION_AMBIGUOUS);
  }
  return uniqueSorted(codes);
}

function dutyTypeForMeasures(
  measures: readonly TariffMeasure[],
): TenDigitCandidate["dutyType"] {
  if (measures.some((measure) => measure.dutyExpression?.includes("+"))) {
    return "compound";
  }
  if (
    measures.some((measure) =>
      /\b(per|\/)\b/i.test(measure.dutyExpression ?? ""),
    )
  ) {
    return "specific";
  }
  if (measures.some((measure) => measure.dutyExpression?.includes("%"))) {
    return "ad_valorem";
  }
  if (measures.length > 0) {
    return "unknown";
  }
  return undefined;
}

function isTradeRemedyMeasure(measure: TariffMeasure): boolean {
  return measure.kind === "anti_dumping";
}

function dedupePersistenceCandidates(
  candidates: RecallLatticePersistenceCandidate[],
): RecallLatticePersistenceCandidate[] {
  const byKey = new Map<string, RecallLatticePersistenceCandidate>();
  for (const candidate of candidates) {
    byKey.set(`${candidate.code}:${candidate.sourceFamily}`, candidate);
  }
  return [...byKey.values()].sort(
    (left, right) =>
      left.rank - right.rank ||
      left.code.localeCompare(right.code) ||
      left.sourceFamily.localeCompare(right.sourceFamily),
  );
}

function productCommodityCodeId(
  productCandidateId: string,
  candidate: RecallLatticePersistenceCandidate,
): string {
  return `pcc_${hashParts([
    productCandidateId,
    candidate.code,
    candidate.sourceFamily,
  ]).slice(0, 24)}`;
}

function commodityDigits(code: string): 6 | 8 | 10 {
  if (code.length === 6 || code.length === 8 || code.length === 10) {
    return code.length;
  }
  throw new Error(`unsupported commodity code digit length: ${code}`);
}

function materialMismatch(node: TariffNode, material: string): boolean {
  const descriptionTokens = tokenize(node.description);
  const materialTokens = tokenize(material);
  if (matchesAny(descriptionTokens, materialTokens)) {
    return false;
  }
  if (matchesAny(tokenize(node.exclusions.join(" ")), materialTokens)) {
    return true;
  }
  return matchesAny(descriptionTokens, ["plastic", "rubber", "steel", "metal", "textile"]);
}

function nounTerms(headNoun: string): string[] {
  const terms = tokenize(headNoun);
  if (terms.includes("cartons") || terms.includes("carton")) {
    return [...terms, "box", "boxes", "case", "cases"];
  }
  if (terms.includes("boxes") || terms.includes("box")) {
    return [...terms, "carton", "cartons"];
  }
  return terms;
}

function matchesAny(haystack: string[], needles: string[]): boolean {
  return needles.some((needle) => haystack.includes(needle));
}

function tokenize(value: string): string[] {
  return value
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((token) => token.length > 1);
}

function clampBps(value: number): number {
  return Math.max(0, Math.min(10_000, value));
}

function uniqueBy<T>(items: T[], key: (item: T) => string): T[] {
  const seen = new Set<string>();
  return items.filter((item) => {
    const itemKey = key(item);
    if (seen.has(itemKey)) {
      return false;
    }
    seen.add(itemKey);
    return true;
  });
}

function uniqueSorted<T extends string>(items: readonly T[]): T[] {
  return [...new Set(items)].sort((left, right) => left.localeCompare(right));
}

function stableJson(value: unknown): string {
  return JSON.stringify(sortJson(value));
}

function sortJson(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(sortJson);
  }
  if (typeof value === "object" && value !== null) {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, nested]) => [key, sortJson(nested)]),
    );
  }
  return value;
}

function hashParts(parts: string[]): string {
  return createHash("sha256").update(parts.join("\0")).digest("hex");
}

function sha256(value: string): string {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}
