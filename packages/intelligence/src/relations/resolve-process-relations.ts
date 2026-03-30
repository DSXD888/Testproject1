export const RELATION_MODEL_VERSION = 1;
export const MERGE_SCORE_THRESHOLD = 0.95;
export const REVIEW_SCORE_THRESHOLD = 0.75;

export const RELATION_SCORE_WEIGHTS = {
  referenceMatch: 0.35,
  buyerMatch: 0.2,
  titleSimilarity: 0.2,
  cpvOverlap: 0.1,
  dateProximity: 0.1,
  valueProximity: 0.05,
} as const;

export type ProcessRelationClassification =
  | "merged_same_process"
  | "related_not_merged"
  | "possible_duplicate"
  | "separate";

export type RelationThresholdBucket =
  | "below_review"
  | "review_candidate"
  | "merge_candidate";

export type RelationBlocker =
  | "reference_conflict"
  | "buyer_mismatch"
  | "framework_calloff_mismatch"
  | "explicit_separate_process"
  | "incompatible_chronology"
  | "lot_or_value_structure_conflict";

export type RelationHint =
  | "linked_notice_chain"
  | "framework_call_off"
  | "planning_tender"
  | "same_subject_same_buyer";

export type ExactMatchRule =
  | "same_ocid"
  | "linked_notice_chain"
  | "framework_call_off"
  | "planning_tender"
  | null;

export type ClusterCachePolicy = "none" | "trust_aware";

export type ProcurementStage =
  | "planning"
  | "tender"
  | "award"
  | "contract"
  | "framework"
  | "call_off"
  | "unknown";

export interface ProcessAliasRedirect {
  aliasProcessId: string;
  canonicalProcessId: string;
  aliasReason: "merged_same_process";
}

export interface RelationHeuristicFeatures {
  referenceMatch: number;
  buyerMatch: number;
  titleSimilarity: number;
  cpvOverlap: number;
  dateProximity: number;
  valueProximity: number;
}

export interface RelationHeuristicOverrides
  extends Partial<RelationHeuristicFeatures> {
  totalScore?: number;
}

export interface ProcurementProcessIdentity {
  id: string;
  canonicalProcessId?: string | null;
  canonicalOcid?: string | null;
  createdAt?: string | null;
  buyerName?: string | null;
  buyerIdentifier?: string | null;
  title: string;
  referenceNumber?: string | null;
  publishedAt?: string | null;
  estimatedValuePence?: number | null;
  lotCount?: number | null;
  cpvCodes?: string[] | null;
  stage?: ProcurementStage | null;
  relationHints?: RelationHint[] | null;
  relatedProcessIds?: string[] | null;
  relatedOcidReferences?: string[] | null;
  explicitSeparateProcess?: boolean | null;
  pendingOperatorIntentCount?: number | null;
  operatorDecisionHeadCount?: number | null;
}

export interface RelationResolutionInput {
  left: ProcurementProcessIdentity;
  right: ProcurementProcessIdentity;
  aliases?: ProcessAliasRedirect[] | null;
  heuristicOverrides?: RelationHeuristicOverrides | null;
  relationModelVersion?: number;
}

export interface ProcessRelationEvidence {
  blockers: RelationBlocker[];
  exactMatchRule: ExactMatchRule;
  heuristicUsed: boolean;
  heuristicScore: number;
  thresholdBucket: RelationThresholdBucket;
  relationModelVersion: number;
  features: RelationHeuristicFeatures;
  relationHints: RelationHint[];
}

export interface ProcessRelationEdgeCandidate {
  processIdA: string;
  processIdB: string;
  relationType: Exclude<ProcessRelationClassification, "separate">;
  confidenceBps: number;
  evidence: ProcessRelationEvidence;
  writeDisposition: "required" | "optional";
}

export interface CanonicalProcessUpdate {
  processId: string;
  canonicalProcessId: string;
}

export interface LateMergeEffect {
  manifestRegenerationRequired: boolean;
  pipelineAssessmentRecomputeRequired: boolean;
  decisionStale: boolean;
  stalePendingOperatorIntents: boolean;
  staleOperatorDecisionHeads: boolean;
  emitIdentityChangedNotification: boolean;
}

export interface RelationClusterView {
  visible: boolean;
  relationType: Exclude<
    ProcessRelationClassification,
    "merged_same_process" | "separate"
  > | null;
  confidenceBps: number;
  cachePolicy: ClusterCachePolicy;
  processIds: string[];
}

export interface RelationResolutionResult {
  classification: ProcessRelationClassification;
  confidenceBps: number;
  canonicalProcessId: string;
  retiredProcessId: string | null;
  relationEdge: ProcessRelationEdgeCandidate | null;
  aliasRedirect: ProcessAliasRedirect | null;
  canonicalUpdates: CanonicalProcessUpdate[];
  operatorReviewRequired: boolean;
  evidence: ProcessRelationEvidence;
  lateMergeEffect: LateMergeEffect | null;
  clusterView: RelationClusterView;
}

export function resolveProcessRelations(
  input: RelationResolutionInput,
): RelationResolutionResult {
  const relationModelVersion =
    input.relationModelVersion ?? RELATION_MODEL_VERSION;
  const leftCanonical = resolveCanonicalProcessId(input.left, input.aliases ?? []);
  const rightCanonical = resolveCanonicalProcessId(input.right, input.aliases ?? []);
  const hints = collectRelationHints(input.left, input.right);
  const blockers = detectRelationBlockers(input.left, input.right);
  const exactMatchRule = detectExactMatchRule(input.left, input.right, hints);
  const features = buildRelationHeuristicFeatures(
    input.left,
    input.right,
    input.heuristicOverrides ?? undefined,
  );
  const heuristicScore = clampScore(
    input.heuristicOverrides?.totalScore ?? computeHeuristicScore(features),
  );
  const thresholdBucket = classifyThresholdBucket(heuristicScore);

  const evidence: ProcessRelationEvidence = {
    blockers,
    exactMatchRule,
    heuristicUsed:
      exactMatchRule === null ||
      (exactMatchRule !== "framework_call_off" &&
        exactMatchRule !== "planning_tender" &&
        blockers.length > 0),
    heuristicScore,
    thresholdBucket,
    relationModelVersion,
    features,
    relationHints: hints,
  };

  const classification = classifyRelation({
    blockers,
    exactMatchRule,
    heuristicScore,
    left: input.left,
    right: input.right,
  });
  const confidenceBps = classifyConfidenceBps(
    classification,
    exactMatchRule,
    heuristicScore,
  );
  const survivorCanonicalProcessId = chooseCanonicalSurvivor(
    input.left,
    input.right,
    leftCanonical,
    rightCanonical,
  );
  const retiredProcessId =
    classification === "merged_same_process"
      ? resolveRetiredProcessId(
          input.left,
          input.right,
          survivorCanonicalProcessId,
          leftCanonical,
          rightCanonical,
        )
      : null;

  const relationEdge = buildRelationEdgeCandidate(
    classification,
    leftCanonical,
    rightCanonical,
    confidenceBps,
    evidence,
  );
  const aliasRedirect =
    retiredProcessId === null
      ? null
      : {
          aliasProcessId: retiredProcessId,
          canonicalProcessId: survivorCanonicalProcessId,
          aliasReason: "merged_same_process" as const,
        };
  const canonicalUpdates =
    classification === "merged_same_process"
      ? [
          {
            processId: input.left.id,
            canonicalProcessId: survivorCanonicalProcessId,
          },
          {
            processId: input.right.id,
            canonicalProcessId: survivorCanonicalProcessId,
          },
        ]
      : [];

  return {
    classification,
    confidenceBps,
    canonicalProcessId:
      classification === "merged_same_process"
        ? survivorCanonicalProcessId
        : leftCanonical,
    retiredProcessId,
    relationEdge,
    aliasRedirect,
    canonicalUpdates,
    operatorReviewRequired: classification === "possible_duplicate",
    evidence,
    lateMergeEffect: buildLateMergeEffect(
      classification,
      input.left,
      input.right,
      leftCanonical,
      rightCanonical,
    ),
    clusterView: buildClusterView(
      classification,
      confidenceBps,
      leftCanonical,
      rightCanonical,
    ),
  };
}

export function resolveCanonicalProcessId(
  process: Pick<ProcurementProcessIdentity, "id" | "canonicalProcessId">,
  aliases: readonly ProcessAliasRedirect[],
): string {
  const seen = new Set<string>();
  let current = process.canonicalProcessId ?? process.id;
  while (!seen.has(current)) {
    seen.add(current);
    const alias = aliases.find((candidate) => candidate.aliasProcessId === current);
    if (!alias) {
      return current;
    }
    current = alias.canonicalProcessId;
  }

  return current;
}

export function buildRelationHeuristicFeatures(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
  overrides?: RelationHeuristicOverrides,
): RelationHeuristicFeatures {
  return {
    referenceMatch: clampScore(
      overrides?.referenceMatch ?? computeReferenceMatch(left, right),
    ),
    buyerMatch: clampScore(overrides?.buyerMatch ?? computeBuyerMatch(left, right)),
    titleSimilarity: clampScore(
      overrides?.titleSimilarity ?? computeTitleSimilarity(left, right),
    ),
    cpvOverlap: clampScore(overrides?.cpvOverlap ?? computeCpvOverlap(left, right)),
    dateProximity: clampScore(
      overrides?.dateProximity ?? computeDateProximity(left, right),
    ),
    valueProximity: clampScore(
      overrides?.valueProximity ?? computeValueProximity(left, right),
    ),
  };
}

export function computeHeuristicScore(
  features: RelationHeuristicFeatures,
): number {
  return clampScore(
    features.referenceMatch * RELATION_SCORE_WEIGHTS.referenceMatch +
      features.buyerMatch * RELATION_SCORE_WEIGHTS.buyerMatch +
      features.titleSimilarity * RELATION_SCORE_WEIGHTS.titleSimilarity +
      features.cpvOverlap * RELATION_SCORE_WEIGHTS.cpvOverlap +
      features.dateProximity * RELATION_SCORE_WEIGHTS.dateProximity +
      features.valueProximity * RELATION_SCORE_WEIGHTS.valueProximity,
  );
}

export function classifyThresholdBucket(score: number): RelationThresholdBucket {
  if (score >= MERGE_SCORE_THRESHOLD) {
    return "merge_candidate";
  }

  if (score >= REVIEW_SCORE_THRESHOLD) {
    return "review_candidate";
  }

  return "below_review";
}

function classifyRelation(input: {
  blockers: RelationBlocker[];
  exactMatchRule: ExactMatchRule;
  heuristicScore: number;
  left: ProcurementProcessIdentity;
  right: ProcurementProcessIdentity;
}): ProcessRelationClassification {
  const { blockers, exactMatchRule, heuristicScore, left, right } = input;
  const hasBlocker = blockers.length > 0;
  const nonMergeRelation = isNonMergeSemanticRelation(exactMatchRule, left, right);

  if (exactMatchRule === "framework_call_off" || exactMatchRule === "planning_tender") {
    return "related_not_merged";
  }

  if (exactMatchRule === "same_ocid" || exactMatchRule === "linked_notice_chain") {
    return hasBlocker ? "possible_duplicate" : "merged_same_process";
  }

  if (heuristicScore < REVIEW_SCORE_THRESHOLD) {
    return "separate";
  }

  if (hasBlocker) {
    return nonMergeRelation ? "related_not_merged" : "possible_duplicate";
  }

  if (heuristicScore >= MERGE_SCORE_THRESHOLD) {
    return "merged_same_process";
  }

  return nonMergeRelation ? "related_not_merged" : "possible_duplicate";
}

function classifyConfidenceBps(
  classification: ProcessRelationClassification,
  exactMatchRule: ExactMatchRule,
  heuristicScore: number,
): number {
  if (classification === "merged_same_process" && exactMatchRule === "same_ocid") {
    return 10000;
  }

  if (classification === "merged_same_process" && exactMatchRule === "linked_notice_chain") {
    return 9800;
  }

  if (classification === "related_not_merged" && exactMatchRule === "framework_call_off") {
    return 9000;
  }

  if (classification === "related_not_merged" && exactMatchRule === "planning_tender") {
    return 8500;
  }

  if (classification === "separate") {
    return Math.round(clampScore(heuristicScore) * 10000);
  }

  return Math.round(clampScore(heuristicScore) * 10000);
}

function buildRelationEdgeCandidate(
  classification: ProcessRelationClassification,
  leftCanonical: string,
  rightCanonical: string,
  confidenceBps: number,
  evidence: ProcessRelationEvidence,
): ProcessRelationEdgeCandidate | null {
  if (classification === "separate") {
    return null;
  }

  return {
    processIdA: leftCanonical,
    processIdB: rightCanonical,
    relationType: classification,
    confidenceBps,
    evidence,
    writeDisposition:
      classification === "merged_same_process" ? "optional" : "required",
  };
}

function buildLateMergeEffect(
  classification: ProcessRelationClassification,
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
  leftCanonical: string,
  rightCanonical: string,
): LateMergeEffect | null {
  if (
    classification !== "merged_same_process" ||
    leftCanonical === rightCanonical
  ) {
    return null;
  }

  const stalePendingOperatorIntents =
    (left.pendingOperatorIntentCount ?? 0) + (right.pendingOperatorIntentCount ?? 0) >
    0;
  const staleOperatorDecisionHeads =
    (left.operatorDecisionHeadCount ?? 0) + (right.operatorDecisionHeadCount ?? 0) >
    0;
  const decisionStale =
    stalePendingOperatorIntents || staleOperatorDecisionHeads;

  return {
    manifestRegenerationRequired: true,
    pipelineAssessmentRecomputeRequired: true,
    decisionStale,
    stalePendingOperatorIntents,
    staleOperatorDecisionHeads,
    emitIdentityChangedNotification: decisionStale,
  };
}

function buildClusterView(
  classification: ProcessRelationClassification,
  confidenceBps: number,
  leftCanonical: string,
  rightCanonical: string,
): RelationClusterView {
  if (
    classification !== "related_not_merged" &&
    classification !== "possible_duplicate"
  ) {
    return {
      visible: false,
      relationType: null,
      confidenceBps,
      cachePolicy: "none",
      processIds: uniqueIds([leftCanonical, rightCanonical]),
    };
  }

  return {
    visible: true,
    relationType: classification,
    confidenceBps,
    cachePolicy: "trust_aware",
    processIds: uniqueIds([leftCanonical, rightCanonical]),
  };
}

function detectRelationBlockers(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): RelationBlocker[] {
  const blockers = new Set<RelationBlocker>();
  const leftReference = normalizeReference(left.referenceNumber);
  const rightReference = normalizeReference(right.referenceNumber);
  const leftCanonicalOcid = normalizeIdentifier(left.canonicalOcid);
  const rightCanonicalOcid = normalizeIdentifier(right.canonicalOcid);

  if (leftReference && rightReference && leftReference !== rightReference) {
    blockers.add("reference_conflict");
  }

  if (
    leftCanonicalOcid &&
    rightCanonicalOcid &&
    leftCanonicalOcid === rightCanonicalOcid &&
    !buyersCompatible(left, right)
  ) {
    blockers.add("buyer_mismatch");
  }

  if (left.explicitSeparateProcess || right.explicitSeparateProcess) {
    blockers.add("explicit_separate_process");
  }

  if (hasFrameworkCallOffMismatch(left, right)) {
    blockers.add("framework_calloff_mismatch");
  }

  if (hasIncompatibleChronology(left, right)) {
    blockers.add("incompatible_chronology");
  }

  if (hasLotOrValueConflict(left, right)) {
    blockers.add("lot_or_value_structure_conflict");
  }

  return [...blockers];
}

function detectExactMatchRule(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
  hints: RelationHint[],
): ExactMatchRule {
  const leftCanonicalOcid = normalizeIdentifier(left.canonicalOcid);
  const rightCanonicalOcid = normalizeIdentifier(right.canonicalOcid);
  if (leftCanonicalOcid && rightCanonicalOcid && leftCanonicalOcid === rightCanonicalOcid) {
    return "same_ocid";
  }

  if (
    hints.includes("linked_notice_chain") ||
    processLinksTo(left, right.id) ||
    processLinksTo(right, left.id) ||
    referencesLinkedOcid(left, rightCanonicalOcid) ||
    referencesLinkedOcid(right, leftCanonicalOcid)
  ) {
    return "linked_notice_chain";
  }

  if (
    hints.includes("framework_call_off") ||
    isFrameworkCallOffPair(left.stage, right.stage)
  ) {
    return "framework_call_off";
  }

  if (
    hints.includes("planning_tender") ||
    isPlanningTenderPair(left.stage, right.stage)
  ) {
    return "planning_tender";
  }

  return null;
}

function chooseCanonicalSurvivor(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
  leftCanonical: string,
  rightCanonical: string,
): string {
  if (leftCanonical === rightCanonical) {
    return leftCanonical;
  }

  const leftCreatedAt = parseTimestamp(left.createdAt);
  const rightCreatedAt = parseTimestamp(right.createdAt);
  if (leftCreatedAt !== null && rightCreatedAt !== null) {
    if (leftCreatedAt < rightCreatedAt) {
      return leftCanonical;
    }

    if (rightCreatedAt < leftCreatedAt) {
      return rightCanonical;
    }
  }

  return leftCanonical.localeCompare(rightCanonical) <= 0
    ? leftCanonical
    : rightCanonical;
}

function resolveRetiredProcessId(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
  survivorCanonicalProcessId: string,
  leftCanonical: string,
  rightCanonical: string,
): string | null {
  if (leftCanonical === rightCanonical) {
    return null;
  }

  if (survivorCanonicalProcessId === leftCanonical) {
    return rightCanonical;
  }

  if (survivorCanonicalProcessId === rightCanonical) {
    return leftCanonical;
  }

  return left.id === survivorCanonicalProcessId ? right.id : left.id;
}

function collectRelationHints(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): RelationHint[] {
  const hints = new Set<RelationHint>();
  for (const hint of left.relationHints ?? []) {
    hints.add(hint);
  }
  for (const hint of right.relationHints ?? []) {
    hints.add(hint);
  }

  if (sameBuyerAndSimilarTitle(left, right)) {
    hints.add("same_subject_same_buyer");
  }

  return [...hints];
}

function isNonMergeSemanticRelation(
  exactMatchRule: ExactMatchRule,
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): boolean {
  if (exactMatchRule === "framework_call_off" || exactMatchRule === "planning_tender") {
    return true;
  }

  return sameBuyerAndSimilarTitle(left, right);
}

function sameBuyerAndSimilarTitle(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): boolean {
  return (
    computeBuyerMatch(left, right) >= 1 &&
    computeTitleSimilarity(left, right) >= 0.8
  );
}

function computeReferenceMatch(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): number {
  const leftReference = normalizeReference(left.referenceNumber);
  const rightReference = normalizeReference(right.referenceNumber);
  if (!leftReference || !rightReference) {
    return 0;
  }

  if (leftReference === rightReference) {
    return 1;
  }

  const leftStem = stripReferenceSuffix(leftReference);
  const rightStem = stripReferenceSuffix(rightReference);
  if (leftStem === rightStem) {
    return 0.9;
  }

  if (leftStem.includes(rightStem) || rightStem.includes(leftStem)) {
    return 0.5;
  }

  return 0;
}

function computeBuyerMatch(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): number {
  const leftBuyerIdentifier = normalizeIdentifier(left.buyerIdentifier);
  const rightBuyerIdentifier = normalizeIdentifier(right.buyerIdentifier);
  if (
    leftBuyerIdentifier &&
    rightBuyerIdentifier &&
    leftBuyerIdentifier === rightBuyerIdentifier
  ) {
    return 1;
  }

  const leftBuyerName = normalizeIdentifier(left.buyerName);
  const rightBuyerName = normalizeIdentifier(right.buyerName);
  if (!leftBuyerName || !rightBuyerName) {
    return 0;
  }

  return leftBuyerName === rightBuyerName ? 1 : 0;
}

function computeTitleSimilarity(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): number {
  return jaccardSimilarity(tokenize(left.title), tokenize(right.title));
}

function computeCpvOverlap(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): number {
  const leftCodes = (left.cpvCodes ?? []).map(normalizeCpvCode).filter(Boolean);
  const rightCodes = (right.cpvCodes ?? []).map(normalizeCpvCode).filter(Boolean);
  return jaccardSimilarity(leftCodes, rightCodes);
}

function computeDateProximity(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): number {
  const leftPublishedAt = parseTimestamp(left.publishedAt);
  const rightPublishedAt = parseTimestamp(right.publishedAt);
  if (leftPublishedAt === null || rightPublishedAt === null) {
    return 0;
  }

  const dayDifference = Math.abs(leftPublishedAt - rightPublishedAt) / 86_400_000;
  if (dayDifference <= 7) {
    return 1;
  }
  if (dayDifference <= 30) {
    return 0.9;
  }
  if (dayDifference <= 90) {
    return 0.75;
  }
  if (dayDifference <= 180) {
    return 0.5;
  }
  if (dayDifference <= 365) {
    return 0.25;
  }

  return 0;
}

function computeValueProximity(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): number {
  const leftValue = left.estimatedValuePence;
  const rightValue = right.estimatedValuePence;
  if (
    leftValue === null ||
    leftValue === undefined ||
    rightValue === null ||
    rightValue === undefined ||
    leftValue < 0 ||
    rightValue < 0
  ) {
    return 0;
  }

  if (leftValue === 0 && rightValue === 0) {
    return 1;
  }

  const larger = Math.max(leftValue, rightValue);
  const smaller = Math.min(leftValue, rightValue);
  if (larger === 0) {
    return 0;
  }

  return smaller / larger;
}

function buyersCompatible(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): boolean {
  return computeBuyerMatch(left, right) >= 1;
}

function hasFrameworkCallOffMismatch(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): boolean {
  if (isFrameworkCallOffPair(left.stage, right.stage)) {
    return true;
  }

  return false;
}

function hasIncompatibleChronology(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): boolean {
  const leftPublishedAt = parseTimestamp(left.publishedAt);
  const rightPublishedAt = parseTimestamp(right.publishedAt);
  if (leftPublishedAt === null || rightPublishedAt === null) {
    return false;
  }

  const leftRank = stageRank(left.stage);
  const rightRank = stageRank(right.stage);
  if (leftRank === rightRank) {
    return false;
  }

  const chronologyInverted =
    (leftRank < rightRank && leftPublishedAt > rightPublishedAt) ||
    (rightRank < leftRank && rightPublishedAt > leftPublishedAt);

  return chronologyInverted && Math.abs(leftPublishedAt - rightPublishedAt) > 86_400_000;
}

function hasLotOrValueConflict(
  left: ProcurementProcessIdentity,
  right: ProcurementProcessIdentity,
): boolean {
  if (!sameBuyerAndSimilarTitle(left, right)) {
    return false;
  }

  const lotConflict =
    left.lotCount !== null &&
    left.lotCount !== undefined &&
    right.lotCount !== null &&
    right.lotCount !== undefined &&
    Math.abs(left.lotCount - right.lotCount) >= 2;

  const leftValue = left.estimatedValuePence;
  const rightValue = right.estimatedValuePence;
  const valueConflict =
    leftValue !== null &&
    leftValue !== undefined &&
    rightValue !== null &&
    rightValue !== undefined &&
    Math.max(leftValue, rightValue) > 0 &&
    Math.max(leftValue, rightValue) / Math.max(1, Math.min(leftValue, rightValue)) >= 3;

  return lotConflict || valueConflict;
}

function processLinksTo(
  process: ProcurementProcessIdentity,
  targetProcessId: string,
): boolean {
  return (process.relatedProcessIds ?? []).includes(targetProcessId);
}

function referencesLinkedOcid(
  process: ProcurementProcessIdentity,
  targetCanonicalOcid: string | null,
): boolean {
  if (!targetCanonicalOcid) {
    return false;
  }

  return (process.relatedOcidReferences ?? []).some(
    (candidate) => normalizeIdentifier(candidate) === targetCanonicalOcid,
  );
}

function isFrameworkCallOffPair(
  leftStage: ProcurementStage | null | undefined,
  rightStage: ProcurementStage | null | undefined,
): boolean {
  const stages = new Set([normalizeStage(leftStage), normalizeStage(rightStage)]);
  return stages.has("framework") && stages.has("call_off");
}

function isPlanningTenderPair(
  leftStage: ProcurementStage | null | undefined,
  rightStage: ProcurementStage | null | undefined,
): boolean {
  const stages = new Set([normalizeStage(leftStage), normalizeStage(rightStage)]);
  return stages.has("planning") && stages.has("tender");
}

function stageRank(stage: ProcurementStage | null | undefined): number {
  switch (normalizeStage(stage)) {
    case "planning":
      return 1;
    case "framework":
      return 2;
    case "tender":
      return 3;
    case "call_off":
      return 4;
    case "award":
      return 5;
    case "contract":
      return 6;
    default:
      return 99;
  }
}

function normalizeStage(
  stage: ProcurementStage | null | undefined,
): ProcurementStage {
  return stage ?? "unknown";
}

function normalizeReference(reference: string | null | undefined): string | null {
  const normalized = normalizeIdentifier(reference);
  return normalized.length === 0 ? null : normalized;
}

function stripReferenceSuffix(reference: string): string {
  return reference.replace(/(?:LOT|LOTNO|AMENDMENT|AMD)\d+$/u, "");
}

function normalizeIdentifier(value: string | null | undefined): string {
  return (value ?? "").toLowerCase().replace(/[^a-z0-9]+/gu, "");
}

function normalizeCpvCode(value: string): string {
  return value.replace(/[^0-9]/gu, "").slice(0, 8);
}

function tokenize(value: string): string[] {
  return value
    .toLowerCase()
    .split(/[^a-z0-9]+/u)
    .filter((token) => token.length > 1);
}

function jaccardSimilarity(
  left: readonly string[],
  right: readonly string[],
): number {
  const leftSet = new Set(left);
  const rightSet = new Set(right);
  if (leftSet.size === 0 || rightSet.size === 0) {
    return 0;
  }

  let intersection = 0;
  for (const token of leftSet) {
    if (rightSet.has(token)) {
      intersection += 1;
    }
  }

  const union = new Set([...leftSet, ...rightSet]).size;
  return union === 0 ? 0 : intersection / union;
}

function parseTimestamp(value: string | null | undefined): number | null {
  if (!value) {
    return null;
  }

  const timestamp = Date.parse(value);
  return Number.isNaN(timestamp) ? null : timestamp;
}

function clampScore(value: number): number {
  if (value <= 0) {
    return 0;
  }

  if (value >= 1) {
    return 1;
  }

  return Number(value.toFixed(4));
}

function uniqueIds(values: readonly string[]): string[] {
  return [...new Set(values)];
}
