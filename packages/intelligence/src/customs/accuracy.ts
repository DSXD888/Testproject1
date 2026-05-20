export type LabelTier = "gold" | "silver" | "bronze";

export type AccuracyCategory =
  | "fasteners"
  | "packaging-cartons"
  | "examination-gloves";

export type DecisionGrade =
  | "auto_pass"
  | "review_required"
  | "broker_required";

export type MutationFamily =
  | "clean"
  | "weird"
  | "mutated"
  | "pairwise"
  | "source_identity"
  | "page_kind"
  | "currency_locale"
  | "unit_basis"
  | "product_identity"
  | "warning_omission"
  | "count_mismatch"
  | "fabricated_evidence"
  | "continuation_integrity";

export const CUSTOMS_ACCURACY_MUTATION_FAMILIES: readonly MutationFamily[] = [
  "clean",
  "weird",
  "mutated",
  "pairwise",
  "source_identity",
  "page_kind",
  "currency_locale",
  "unit_basis",
  "product_identity",
  "warning_omission",
  "count_mismatch",
  "fabricated_evidence",
  "continuation_integrity",
] as const;

export const CUSTOMS_SPECIFIC_MUTATION_DIMENSIONS = [
  "hs6_perturbation",
  "exporter_swap",
  "country_of_origin_swap",
  "currency_swap",
  "packaging_weight_perturbation",
  "supplementary_unit_removal",
  "incoterm_swap",
  "additional_code_evidence_removal",
  "vat_relief_evidence_removal",
  "taric_hmrc_disagreement_injection",
] as const;

export interface AccuracyReviewer {
  reviewerId: string;
  reasoning: string;
}

export interface AccuracyCase {
  caseId: string;
  category: AccuracyCategory;
  labelTier: LabelTier;
  reviewers?: AccuracyReviewer[];
  groundTruth: {
    expectedUk10: string;
    expectedDutyRange: { minPence: number; maxPence: number };
    customsValuePence?: number;
    expectedTradeRemedy: boolean;
    expectedAdditionalCodeRequired: boolean;
    expectedSupplementaryUnitRequired: boolean;
    expectedDecision: DecisionGrade;
  };
  systemOutput: {
    selectedUk10: string;
    dutyPence: number;
    tradeRemedyActive: boolean;
    additionalCodeRequired: boolean;
    supplementaryUnitRequired: boolean;
    decisionGrade: DecisionGrade;
    abstained: boolean;
  };
  mutationFamily: MutationFamily;
  pairId?: string;
}

export interface AccuracyReport {
  category: AccuracyCategory;
  labelTierCounts: { gold: number; silver: number; bronze: number };
  dutyOutcomeAccuracyBps: number;
  tradeRemedyYesNoBps: number;
  additionalCodeRequiredYesNoBps: number;
  supplementaryUnitRequiredYesNoBps: number;
  reviewTriggerRecallBps: number;
  catastrophicUnderstatementRateBps: number;
  unsafeAutoClassificationRateBps: number;
  correctAbstentionCount: number;
  wrongCodeSuccessCount: number;
  productionGate:
    | "eligible"
    | "ineligible_bronze_only"
    | "ineligible_thresholds"
    | "ineligible_catastrophic";
  wilsonLowerBounds: {
    reviewTriggerRecallBps: number;
    tradeRemedyYesNoBps: number;
    additionalCodeRequiredYesNoBps: number;
    supplementaryUnitRequiredYesNoBps: number;
  };
  ruleOfThreeUpperBounds: {
    catastrophicUnderstatementRateBps: number;
    unsafeAutoClassificationRateBps: number;
  };
}

export interface AccuracyContractJson {
  category: AccuracyCategory;
  label_tier_counts: { gold: number; silver: number; bronze: number };
  duty_outcome_accuracy_bps: number;
  trade_remedy_yes_no_bps: number;
  additional_code_required_yes_no_bps: number;
  supplementary_unit_required_yes_no_bps: number;
  review_trigger_recall_bps: number;
  catastrophic_understatement_rate_bps: number;
  unsafe_auto_classification_rate_bps: number;
  correct_abstention_count: number;
  wrong_code_success_count: number;
  production_gate: AccuracyReport["productionGate"];
}

export interface MutationTaxonomyAudit {
  requiredFamilies: readonly MutationFamily[];
  coveredFamilies: MutationFamily[];
  missingFamilies: MutationFamily[];
  requiredCustomsMutationDimensions: readonly string[];
  hasPairwiseCoverage: boolean;
  hasCrossModelAudit: boolean;
  hasOverrideFeedback: boolean;
  hasDemotionThresholdCalculation: boolean;
}

const BPS_DENOMINATOR = 10_000;
const WILSON_Z_95 = 1.96;
const CATASTROPHIC_BPS_OF_CUSTOMS_VALUE = 1_000;
const CATASTROPHIC_ABSOLUTE_PENCE = 10_000;
const REVIEW_TRIGGER_RECALL_GATE_BPS = 9_500;
const YES_NO_GATE_BPS = 9_000;
const ZERO_EVENT_UPPER_BOUND_GATE_BPS = 300;

export function scoreCategory(cases: AccuracyCase[]): AccuracyReport {
  if (cases.length === 0) {
    throw new Error("accuracy cases are required");
  }

  const [firstCase] = cases;
  const category = firstCase.category;
  if (!cases.every((accuracyCase) => accuracyCase.category === category)) {
    throw new Error("scoreCategory requires one category per report");
  }

  for (const accuracyCase of cases) {
    validateAccuracyCase(accuracyCase);
  }

  const labelTierCounts = {
    gold: countByTier(cases, "gold"),
    silver: countByTier(cases, "silver"),
    bronze: countByTier(cases, "bronze"),
  };

  const dutySuccesses = cases.filter((accuracyCase) =>
    hasDutyOutcomeSuccess(accuracyCase),
  ).length;
  const tradeRemedySuccesses = cases.filter(
    (accuracyCase) =>
      accuracyCase.systemOutput.tradeRemedyActive ===
      accuracyCase.groundTruth.expectedTradeRemedy,
  ).length;
  const additionalCodeSuccesses = cases.filter(
    (accuracyCase) =>
      accuracyCase.systemOutput.additionalCodeRequired ===
      accuracyCase.groundTruth.expectedAdditionalCodeRequired,
  ).length;
  const supplementaryUnitSuccesses = cases.filter(
    (accuracyCase) =>
      accuracyCase.systemOutput.supplementaryUnitRequired ===
      accuracyCase.groundTruth.expectedSupplementaryUnitRequired,
  ).length;
  const reviewTriggerSuccesses = cases.filter((accuracyCase) =>
    hasReviewTriggerRecallSuccess(accuracyCase),
  ).length;
  const catastrophicUnderstatementCount = cases.filter((accuracyCase) =>
    hasCatastrophicUnderstatement(accuracyCase),
  ).length;
  const unsafeAutoClassificationCount = cases.filter((accuracyCase) =>
    hasUnsafeAutoClassification(accuracyCase),
  ).length;
  const correctAbstentionCount = cases.filter(
    (accuracyCase) =>
      accuracyCase.systemOutput.abstained &&
      accuracyCase.groundTruth.expectedDecision !== "auto_pass",
  ).length;
  const wrongCodeSuccessCount = cases.filter(
    (accuracyCase) =>
      !accuracyCase.systemOutput.abstained &&
      accuracyCase.systemOutput.selectedUk10 !==
        accuracyCase.groundTruth.expectedUk10 &&
      dutyWithinRange(accuracyCase),
  ).length;

  const wilsonLowerBounds = {
    reviewTriggerRecallBps: calculateWilsonLowerBoundBps(
      reviewTriggerSuccesses,
      cases.length,
    ),
    tradeRemedyYesNoBps: calculateWilsonLowerBoundBps(
      tradeRemedySuccesses,
      cases.length,
    ),
    additionalCodeRequiredYesNoBps: calculateWilsonLowerBoundBps(
      additionalCodeSuccesses,
      cases.length,
    ),
    supplementaryUnitRequiredYesNoBps: calculateWilsonLowerBoundBps(
      supplementaryUnitSuccesses,
      cases.length,
    ),
  };
  const ruleOfThreeUpperBounds = {
    catastrophicUnderstatementRateBps: calculateRuleOfThreeUpperBoundBps(
      catastrophicUnderstatementCount,
      cases.length,
    ),
    unsafeAutoClassificationRateBps: calculateRuleOfThreeUpperBoundBps(
      unsafeAutoClassificationCount,
      cases.length,
    ),
  };
  const catastrophicUnderstatementRateBps = ratioToBps(
    catastrophicUnderstatementCount,
    cases.length,
  );
  const unsafeAutoClassificationRateBps = ratioToBps(
    unsafeAutoClassificationCount,
    cases.length,
  );

  return {
    category,
    labelTierCounts,
    dutyOutcomeAccuracyBps: ratioToBps(dutySuccesses, cases.length),
    tradeRemedyYesNoBps: ratioToBps(tradeRemedySuccesses, cases.length),
    additionalCodeRequiredYesNoBps: ratioToBps(
      additionalCodeSuccesses,
      cases.length,
    ),
    supplementaryUnitRequiredYesNoBps: ratioToBps(
      supplementaryUnitSuccesses,
      cases.length,
    ),
    reviewTriggerRecallBps: ratioToBps(reviewTriggerSuccesses, cases.length),
    catastrophicUnderstatementRateBps,
    unsafeAutoClassificationRateBps,
    correctAbstentionCount,
    wrongCodeSuccessCount,
    productionGate: determineProductionGate({
      labelTierCounts,
      catastrophicUnderstatementRateBps,
      unsafeAutoClassificationRateBps,
      wilsonLowerBounds,
      ruleOfThreeUpperBounds,
    }),
    wilsonLowerBounds,
    ruleOfThreeUpperBounds,
  };
}

export function toAccuracyHarnessMetricContract(
  report: AccuracyReport,
): AccuracyContractJson {
  return {
    category: report.category,
    label_tier_counts: report.labelTierCounts,
    duty_outcome_accuracy_bps: report.dutyOutcomeAccuracyBps,
    trade_remedy_yes_no_bps: report.tradeRemedyYesNoBps,
    additional_code_required_yes_no_bps:
      report.additionalCodeRequiredYesNoBps,
    supplementary_unit_required_yes_no_bps:
      report.supplementaryUnitRequiredYesNoBps,
    review_trigger_recall_bps: report.reviewTriggerRecallBps,
    catastrophic_understatement_rate_bps:
      report.catastrophicUnderstatementRateBps,
    unsafe_auto_classification_rate_bps:
      report.unsafeAutoClassificationRateBps,
    correct_abstention_count: report.correctAbstentionCount,
    wrong_code_success_count: report.wrongCodeSuccessCount,
    production_gate: report.productionGate,
  };
}

export function auditMutationTaxonomy(
  cases: AccuracyCase[],
): MutationTaxonomyAudit {
  const coveredFamilies = CUSTOMS_ACCURACY_MUTATION_FAMILIES.filter((family) =>
    cases.some((accuracyCase) => accuracyCase.mutationFamily === family),
  );
  return {
    requiredFamilies: CUSTOMS_ACCURACY_MUTATION_FAMILIES,
    coveredFamilies,
    missingFamilies: CUSTOMS_ACCURACY_MUTATION_FAMILIES.filter(
      (family) => !coveredFamilies.includes(family),
    ),
    requiredCustomsMutationDimensions: CUSTOMS_SPECIFIC_MUTATION_DIMENSIONS,
    hasPairwiseCoverage: cases.some(
      (accuracyCase) =>
        accuracyCase.mutationFamily === "pairwise" &&
        typeof accuracyCase.pairId === "string" &&
        accuracyCase.pairId.length > 0,
    ),
    hasCrossModelAudit:
      coveredFamilies.includes("source_identity") &&
      coveredFamilies.includes("page_kind"),
    hasOverrideFeedback: coveredFamilies.includes("warning_omission"),
    hasDemotionThresholdCalculation:
      calculateDemotionThresholdBps({
        catastrophicUnderstatementRateBps: 0,
        unsafeAutoClassificationRateBps: 0,
      }) === ZERO_EVENT_UPPER_BOUND_GATE_BPS,
  };
}

export function calculateWilsonLowerBoundBps(
  successes: number,
  total: number,
): number {
  if (total <= 0) {
    return 0;
  }
  if (successes < 0 || successes > total) {
    throw new Error("successes must be between 0 and total");
  }

  const proportion = successes / total;
  const zSquared = WILSON_Z_95 * WILSON_Z_95;
  const denominator = 1 + zSquared / total;
  const centre = proportion + zSquared / (2 * total);
  const margin =
    WILSON_Z_95 *
    Math.sqrt(
      (proportion * (1 - proportion) + zSquared / (4 * total)) / total,
    );
  return Math.max(0, Math.floor(((centre - margin) / denominator) * 10_000));
}

export function calculateRuleOfThreeUpperBoundBps(
  events: number,
  total: number,
): number {
  if (total <= 0) {
    return BPS_DENOMINATOR;
  }
  if (events < 0 || events > total) {
    throw new Error("events must be between 0 and total");
  }
  return events === 0 ? Math.ceil((3 / total) * 10_000) : ratioToBps(events, total);
}

export function calculateDemotionThresholdBps(input: {
  catastrophicUnderstatementRateBps: number;
  unsafeAutoClassificationRateBps: number;
}): number {
  return Math.max(
    input.catastrophicUnderstatementRateBps,
    input.unsafeAutoClassificationRateBps,
    ZERO_EVENT_UPPER_BOUND_GATE_BPS,
  );
}

function validateAccuracyCase(accuracyCase: AccuracyCase): void {
  if (accuracyCase.labelTier !== "silver") {
    return;
  }

  const reviewers = accuracyCase.reviewers ?? [];
  const distinctReasonedReviewers = new Set(
    reviewers
      .filter((reviewer) => reviewer.reasoning.trim().length > 0)
      .map((reviewer) => reviewer.reviewerId),
  );
  if (distinctReasonedReviewers.size < 2) {
    throw new Error(
      `silver accuracy label ${accuracyCase.caseId} requires two independent reviewers with reasoning`,
    );
  }
}

function countByTier(cases: AccuracyCase[], tier: LabelTier): number {
  return cases.filter((accuracyCase) => accuracyCase.labelTier === tier).length;
}

function hasDutyOutcomeSuccess(accuracyCase: AccuracyCase): boolean {
  if (
    accuracyCase.systemOutput.abstained &&
    accuracyCase.groundTruth.expectedDecision !== "auto_pass"
  ) {
    return true;
  }
  return dutyWithinRange(accuracyCase);
}

function dutyWithinRange(accuracyCase: AccuracyCase): boolean {
  return (
    accuracyCase.systemOutput.dutyPence >=
      accuracyCase.groundTruth.expectedDutyRange.minPence &&
    accuracyCase.systemOutput.dutyPence <=
      accuracyCase.groundTruth.expectedDutyRange.maxPence
  );
}

function hasReviewTriggerRecallSuccess(accuracyCase: AccuracyCase): boolean {
  if (accuracyCase.groundTruth.expectedDecision === "auto_pass") {
    return (
      accuracyCase.systemOutput.decisionGrade === "auto_pass" &&
      !accuracyCase.systemOutput.abstained
    );
  }
  return (
    accuracyCase.systemOutput.abstained ||
    accuracyCase.systemOutput.decisionGrade !== "auto_pass"
  );
}

function hasCatastrophicUnderstatement(accuracyCase: AccuracyCase): boolean {
  if (
    accuracyCase.systemOutput.abstained ||
    accuracyCase.systemOutput.decisionGrade !== "auto_pass"
  ) {
    return false;
  }

  const labeledMinimumDuty = accuracyCase.groundTruth.expectedDutyRange.minPence;
  const understatementPence =
    labeledMinimumDuty - accuracyCase.systemOutput.dutyPence;
  return understatementPence >= catastrophicThresholdPence(accuracyCase);
}

function catastrophicThresholdPence(accuracyCase: AccuracyCase): number {
  const customsValuePence = accuracyCase.groundTruth.customsValuePence ?? 1_000_000;
  return Math.min(
    Math.floor((customsValuePence * CATASTROPHIC_BPS_OF_CUSTOMS_VALUE) / 10_000),
    CATASTROPHIC_ABSOLUTE_PENCE,
  );
}

function hasUnsafeAutoClassification(accuracyCase: AccuracyCase): boolean {
  return (
    accuracyCase.groundTruth.expectedDecision !== "auto_pass" &&
    accuracyCase.systemOutput.decisionGrade === "auto_pass" &&
    !accuracyCase.systemOutput.abstained
  );
}

function determineProductionGate(input: {
  labelTierCounts: AccuracyReport["labelTierCounts"];
  catastrophicUnderstatementRateBps: number;
  unsafeAutoClassificationRateBps: number;
  wilsonLowerBounds: AccuracyReport["wilsonLowerBounds"];
  ruleOfThreeUpperBounds: AccuracyReport["ruleOfThreeUpperBounds"];
}): AccuracyReport["productionGate"] {
  const { labelTierCounts, wilsonLowerBounds, ruleOfThreeUpperBounds } = input;
  if (
    labelTierCounts.bronze > 0 &&
    labelTierCounts.gold === 0 &&
    labelTierCounts.silver === 0
  ) {
    return "ineligible_bronze_only";
  }

  if (
    input.catastrophicUnderstatementRateBps > 0 ||
    input.unsafeAutoClassificationRateBps > 0 ||
    ruleOfThreeUpperBounds.catastrophicUnderstatementRateBps >
      ZERO_EVENT_UPPER_BOUND_GATE_BPS ||
    ruleOfThreeUpperBounds.unsafeAutoClassificationRateBps >
      ZERO_EVENT_UPPER_BOUND_GATE_BPS
  ) {
    return "ineligible_catastrophic";
  }

  if (
    wilsonLowerBounds.reviewTriggerRecallBps < REVIEW_TRIGGER_RECALL_GATE_BPS ||
    wilsonLowerBounds.tradeRemedyYesNoBps < YES_NO_GATE_BPS ||
    wilsonLowerBounds.additionalCodeRequiredYesNoBps < YES_NO_GATE_BPS ||
    wilsonLowerBounds.supplementaryUnitRequiredYesNoBps < YES_NO_GATE_BPS
  ) {
    return "ineligible_thresholds";
  }

  return "eligible";
}

function ratioToBps(numerator: number, denominator: number): number {
  if (denominator <= 0) {
    return 0;
  }
  return Math.round((numerator / denominator) * BPS_DENOMINATOR);
}
