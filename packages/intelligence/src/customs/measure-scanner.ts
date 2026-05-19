import type { ExporterResolutionResult } from "./exporter-resolution";

export const MEASURE_INTERACTION_UNKNOWN = "MEASURE_INTERACTION_UNKNOWN" as const;
export const SUPPLEMENTARY_UNIT_MISSING = "SUPPLEMENTARY_UNIT_MISSING" as const;

export type InteractionFamily =
  | "additive"
  | "substitutive"
  | "conditional"
  | "documentary_only"
  | "unknown";

export type SupplementaryDutyUnit =
  | "kg"
  | "litre"
  | "pair"
  | "item"
  | "per_1000";

export type SupplementaryQuantityKey =
  | "kg_net"
  | "litre"
  | "pair"
  | "item"
  | "per_1000";

export type AdditionalCodeSatisfiability =
  | "satisfied"
  | "unsatisfied"
  | "unknown";

export type DutyExpression =
  | { kind: "ad_valorem"; rateBps: number }
  | { kind: "specific"; unit: SupplementaryDutyUnit; amountPence: number }
  | { kind: "compound"; components: DutyExpression[] }
  | { kind: "unknown" };

export interface ActiveMeasure {
  id?: string;
  measureType: string;
  dutyExpression: DutyExpression;
  conditions?: {
    quotaCode?: string;
    quotaProofAvailable?: boolean;
    additionalCode?: string;
    additionalCodeSatisfiability?: AdditionalCodeSatisfiability;
    evidenceRequired?: string[];
    inQuotaDutyExpression?: DutyExpression;
    outOfQuotaDutyExpression?: DutyExpression;
    applyWhenAdditionalCodeSatisfied?: DutyExpression;
    rateWithoutAdditionalCode?: DutyExpression;
    documentaryOnly?: boolean;
    interactionFamily?: InteractionFamily;
  };
}

export interface MeasureScanResult {
  perMeasureClassification: Array<{
    measure: ActiveMeasure;
    family: InteractionFamily;
  }>;
  selectedDutyExpression: DutyExpression;
  selectedDutyPence: number | null;
  warnings: string[];
  blockers: string[];
  dutyBreakdownJson: DutyBreakdownPayload;
  measureInteractionJson: MeasureInteractionPayload;
  supplementaryUnitsRequiredJson: SupplementaryQuantityKey[];
  additionalCodesRequirementsJson: AdditionalCodeRequirementPayload[];
}

export interface MeasureScanInput {
  uk10: string;
  measures: ActiveMeasure[];
  customsValuePence: number;
  supplementaryQuantities: Partial<Record<SupplementaryQuantityKey, number>>;
  exporterResolution: ExporterResolutionResult;
  classificationFlags?: {
    gir3Candidate?: boolean;
  };
}

export interface DutyBreakdownPayload {
  kind: "duty_breakdown";
  uk10: string;
  customsValuePence: number;
  components: DutyComponentPayload[];
  substitutiveGroups: SubstitutiveGroupPayload[];
  totalDutyPence: number | null;
}

export interface DutyComponentPayload {
  measureId: string;
  measureType: string;
  family: InteractionFamily;
  expression: DutyExpressionPayload;
  amountPence: number | null;
  selected: boolean;
  blockers: string[];
}

export interface DutyExpressionPayload {
  kind: DutyExpression["kind"];
  rateBps?: number;
  unit?: SupplementaryQuantityKey;
  quantity?: number;
  amountPence: number | null;
  components?: DutyExpressionPayload[];
}

export interface SubstitutiveGroupPayload {
  rule: "greater_of";
  selectedMeasureId: string | null;
  components: Array<{
    measureId: string;
    amountPence: number | null;
    selected: boolean;
  }>;
}

export interface MeasureInteractionPayload {
  kind: "measure_interaction";
  uk10: string;
  exporterRateBasis: ExporterResolutionResult["exporterRateBasis"];
  perMeasure: Array<{
    measureId: string;
    measureType: string;
    family: InteractionFamily;
  }>;
  conditions: ConditionalSelectionPayload[];
  gir3ManualClassificationRequired: boolean;
  blockers: string[];
  warnings: string[];
}

export interface ConditionalSelectionPayload {
  measureId: string;
  quotaCode?: string;
  additionalCode?: string;
  satisfiability?: AdditionalCodeSatisfiability;
  selectedBranch:
    | "in_quota"
    | "out_of_quota"
    | "with_additional_code"
    | "without_additional_code";
  evidenceRequired: string[];
}

export interface AdditionalCodeRequirementPayload {
  code: string;
  satisfiability: AdditionalCodeSatisfiability;
  evidenceRequired: string[];
}

interface EvaluatedMeasure {
  measure: ActiveMeasure;
  measureId: string;
  family: InteractionFamily;
  expression: DutyExpression;
  condition?: ConditionalSelectionPayload;
  amountPence: number | null;
  expressionPayload: DutyExpressionPayload;
  requiredUnits: SupplementaryQuantityKey[];
  blockers: string[];
}

export function scanMeasures(input: MeasureScanInput): MeasureScanResult {
  const perMeasureClassification = input.measures.map((measure) => ({
    measure,
    family: classifyMeasure(measure, input.measures),
  }));

  const evaluatedMeasures = perMeasureClassification.map((entry, index) =>
    evaluateMeasure(entry.measure, entry.family, input, index),
  );

  const substitutiveMeasures = evaluatedMeasures.filter(
    (entry) => entry.family === "substitutive",
  );
  const substitutiveGroup = buildSubstitutiveGroup(substitutiveMeasures);
  const selectedSubstitutiveIds = new Set(
    substitutiveGroup?.selectedMeasureId === null ||
      substitutiveGroup?.selectedMeasureId === undefined
      ? []
      : [substitutiveGroup.selectedMeasureId],
  );
  const additiveTotal = sumSelectedAdditiveDuty(evaluatedMeasures);
  const substitutiveTotal =
    substitutiveGroup?.components.find((component) => component.selected)
      ?.amountPence ?? 0;
  const blockers = unique([
    ...evaluatedMeasures.flatMap((entry) => entry.blockers),
    ...(input.classificationFlags?.gir3Candidate === true
      ? ["gir_3_manual_classification_required"]
      : []),
  ]);
  const hasBlockingDutyGap =
    blockers.includes(MEASURE_INTERACTION_UNKNOWN) ||
    blockers.includes(SUPPLEMENTARY_UNIT_MISSING) ||
    blockers.includes("gir_3_manual_classification_required");
  const selectedDutyPence = hasBlockingDutyGap
    ? null
    : additiveTotal + substitutiveTotal;

  const dutyComponents = evaluatedMeasures.map((entry) => {
    const selected =
      entry.family === "substitutive"
        ? selectedSubstitutiveIds.has(entry.measureId)
        : entry.family !== "documentary_only";
    return {
      measureId: entry.measureId,
      measureType: entry.measure.measureType,
      family: entry.family,
      expression: entry.expressionPayload,
      amountPence: entry.amountPence,
      selected,
      blockers: entry.blockers,
    };
  });

  const warnings = unique(
    evaluatedMeasures.flatMap((entry) => conditionalWarnings(entry)),
  );
  const conditions = evaluatedMeasures.flatMap((entry) =>
    entry.condition === undefined ? [] : [entry.condition],
  );

  return {
    perMeasureClassification,
    selectedDutyExpression: buildSelectedDutyExpression(
      evaluatedMeasures,
      selectedSubstitutiveIds,
    ),
    selectedDutyPence,
    warnings,
    blockers,
    dutyBreakdownJson: {
      kind: "duty_breakdown",
      uk10: input.uk10,
      customsValuePence: input.customsValuePence,
      components: dutyComponents,
      substitutiveGroups: substitutiveGroup === undefined ? [] : [substitutiveGroup],
      totalDutyPence: selectedDutyPence,
    },
    measureInteractionJson: {
      kind: "measure_interaction",
      uk10: input.uk10,
      exporterRateBasis: input.exporterResolution.exporterRateBasis,
      perMeasure: perMeasureClassification.map((entry, index) => ({
        measureId: measureId(entry.measure, index),
        measureType: entry.measure.measureType,
        family: entry.family,
      })),
      conditions,
      gir3ManualClassificationRequired:
        input.classificationFlags?.gir3Candidate === true,
      blockers,
      warnings,
    },
    supplementaryUnitsRequiredJson: unique(
      evaluatedMeasures.flatMap((entry) => entry.requiredUnits),
    ).sort(),
    additionalCodesRequirementsJson: conditions.flatMap((condition) =>
      condition.additionalCode === undefined
        ? []
        : [
            {
              code: condition.additionalCode,
              satisfiability: condition.satisfiability ?? "unknown",
              evidenceRequired: condition.evidenceRequired,
            },
          ],
    ),
  };
}

function classifyMeasure(
  measure: ActiveMeasure,
  allMeasures: readonly ActiveMeasure[],
): InteractionFamily {
  if (measure.conditions?.interactionFamily !== undefined) {
    return measure.conditions.interactionFamily;
  }

  if (measure.dutyExpression.kind === "unknown") {
    return "unknown";
  }

  const normalizedType = normalizeMeasureType(measure.measureType);
  if (
    measure.conditions?.documentaryOnly === true ||
    normalizedType.includes("documentary") ||
    normalizedType.includes("licence") ||
    normalizedType.includes("license")
  ) {
    return "documentary_only";
  }

  if (
    measure.conditions?.quotaCode !== undefined ||
    measure.conditions?.additionalCode !== undefined ||
    normalizedType.includes("quota") ||
    normalizedType.includes("trq") ||
    normalizedType.includes("additional_code")
  ) {
    return "conditional";
  }

  if (normalizedType.includes("unknown") || normalizedType.includes("unmapped")) {
    return "unknown";
  }

  if (isSubstitutiveMeasure(normalizedType, allMeasures)) {
    return "substitutive";
  }

  return "additive";
}

function evaluateMeasure(
  measure: ActiveMeasure,
  family: InteractionFamily,
  input: MeasureScanInput,
  index: number,
): EvaluatedMeasure {
  const id = measureId(measure, index);
  const conditionalExpression = selectConditionalExpression(measure, id);
  const expression = conditionalExpression?.expression ?? measure.dutyExpression;
  const evaluatedExpression = evaluateDutyExpression(
    expression,
    input.customsValuePence,
    input.supplementaryQuantities,
  );
  const blockers = [...evaluatedExpression.blockers];

  if (family === "unknown") {
    blockers.push(MEASURE_INTERACTION_UNKNOWN);
  }

  return {
    measure,
    measureId: id,
    family,
    expression,
    condition: conditionalExpression?.condition,
    amountPence:
      family === "documentary_only" || family === "unknown"
        ? null
        : evaluatedExpression.amountPence,
    expressionPayload: evaluatedExpression.payload,
    requiredUnits: evaluatedExpression.requiredUnits,
    blockers: unique(blockers),
  };
}

function evaluateDutyExpression(
  expression: DutyExpression,
  customsValuePence: number,
  quantities: Partial<Record<SupplementaryQuantityKey, number>>,
): {
  amountPence: number | null;
  payload: DutyExpressionPayload;
  requiredUnits: SupplementaryQuantityKey[];
  blockers: string[];
} {
  switch (expression.kind) {
    case "ad_valorem": {
      const amountPence = bpsDuty(customsValuePence, expression.rateBps);
      return {
        amountPence,
        payload: {
          kind: "ad_valorem",
          rateBps: expression.rateBps,
          amountPence,
        },
        requiredUnits: [],
        blockers: [],
      };
    }
    case "specific": {
      const unit = canonicalSupplementaryUnit(expression.unit);
      const quantity = quantities[unit];
      if (quantity === undefined) {
        return {
          amountPence: null,
          payload: {
            kind: "specific",
            unit,
            amountPence: null,
          },
          requiredUnits: [unit],
          blockers: [SUPPLEMENTARY_UNIT_MISSING],
        };
      }

      const amountPence =
        unit === "per_1000"
          ? Math.round((expression.amountPence * quantity) / 1000)
          : Math.round(expression.amountPence * quantity);
      return {
        amountPence,
        payload: {
          kind: "specific",
          unit,
          quantity,
          amountPence,
        },
        requiredUnits: [unit],
        blockers: [],
      };
    }
    case "compound": {
      const evaluatedComponents = expression.components.map((component) =>
        evaluateDutyExpression(component, customsValuePence, quantities),
      );
      const blockers = unique(
        evaluatedComponents.flatMap((component) => component.blockers),
      );
      const requiredUnits = unique(
        evaluatedComponents.flatMap((component) => component.requiredUnits),
      );
      const amountPence =
        blockers.length > 0
          ? null
          : evaluatedComponents.reduce(
              (sum, component) => sum + (component.amountPence ?? 0),
              0,
            );
      return {
        amountPence,
        payload: {
          kind: "compound",
          amountPence,
          components: evaluatedComponents.map((component) => component.payload),
        },
        requiredUnits,
        blockers,
      };
    }
    case "unknown":
      return {
        amountPence: null,
        payload: {
          kind: "unknown",
          amountPence: null,
        },
        requiredUnits: [],
        blockers: [MEASURE_INTERACTION_UNKNOWN],
      };
  }
}

function selectConditionalExpression(
  measure: ActiveMeasure,
  id: string,
):
  | {
      expression: DutyExpression;
      condition: ConditionalSelectionPayload;
    }
  | undefined {
  const conditions = measure.conditions;
  if (conditions === undefined) {
    return undefined;
  }

  if (conditions.quotaCode !== undefined) {
    const hasProof = conditions.quotaProofAvailable === true;
    const expression = hasProof
      ? (conditions.inQuotaDutyExpression ?? measure.dutyExpression)
      : (conditions.outOfQuotaDutyExpression ?? measure.dutyExpression);
    return {
      expression,
      condition: {
        measureId: id,
        quotaCode: conditions.quotaCode,
        selectedBranch: hasProof ? "in_quota" : "out_of_quota",
        evidenceRequired: conditions.evidenceRequired ?? ["quota_allocation_proof"],
      },
    };
  }

  if (conditions.additionalCode !== undefined) {
    const satisfiability = conditions.additionalCodeSatisfiability ?? "unknown";
    const satisfied = satisfiability === "satisfied";
    const expression = satisfied
      ? (conditions.applyWhenAdditionalCodeSatisfied ?? measure.dutyExpression)
      : (conditions.rateWithoutAdditionalCode ?? measure.dutyExpression);
    return {
      expression,
      condition: {
        measureId: id,
        additionalCode: conditions.additionalCode,
        satisfiability,
        selectedBranch: satisfied
          ? "with_additional_code"
          : "without_additional_code",
        evidenceRequired: conditions.evidenceRequired ?? ["additional_code_proof"],
      },
    };
  }

  return undefined;
}

function buildSubstitutiveGroup(
  substitutiveMeasures: readonly EvaluatedMeasure[],
): SubstitutiveGroupPayload | undefined {
  if (substitutiveMeasures.length === 0) {
    return undefined;
  }

  const selected = substitutiveMeasures.reduce<EvaluatedMeasure | null>(
    (current, candidate) => {
      if (candidate.amountPence === null) {
        return current;
      }

      if (current === null || (current.amountPence ?? 0) < candidate.amountPence) {
        return candidate;
      }

      return current;
    },
    null,
  );

  return {
    rule: "greater_of",
    selectedMeasureId: selected?.measureId ?? null,
    components: substitutiveMeasures.map((entry) => ({
      measureId: entry.measureId,
      amountPence: entry.amountPence,
      selected: selected?.measureId === entry.measureId,
    })),
  };
}

function sumSelectedAdditiveDuty(evaluatedMeasures: readonly EvaluatedMeasure[]): number {
  return evaluatedMeasures.reduce((sum, entry) => {
    if (entry.family === "additive" || entry.family === "conditional") {
      return sum + (entry.amountPence ?? 0);
    }

    return sum;
  }, 0);
}

function buildSelectedDutyExpression(
  evaluatedMeasures: readonly EvaluatedMeasure[],
  selectedSubstitutiveIds: ReadonlySet<string>,
): DutyExpression {
  const selectedExpressions = evaluatedMeasures.flatMap((entry) => {
    if (entry.family === "additive" || entry.family === "conditional") {
      return [entry.expression];
    }

    if (
      entry.family === "substitutive" &&
      selectedSubstitutiveIds.has(entry.measureId)
    ) {
      return [entry.expression];
    }

    return [];
  });

  if (selectedExpressions.length === 0) {
    return { kind: "unknown" };
  }

  if (selectedExpressions.length === 1) {
    return selectedExpressions[0] ?? { kind: "unknown" };
  }

  return {
    kind: "compound",
    components: selectedExpressions,
  };
}

function conditionalWarnings(entry: EvaluatedMeasure): string[] {
  if (entry.condition?.selectedBranch === "out_of_quota") {
    return [`quota:${entry.condition.quotaCode}:out_of_quota_default`];
  }

  if (entry.condition?.selectedBranch === "without_additional_code") {
    return [`additional_code:${entry.condition.additionalCode}:not_claimed`];
  }

  return [];
}

function isSubstitutiveMeasure(
  normalizedType: string,
  allMeasures: readonly ActiveMeasure[],
): boolean {
  const hasSafeguard = allMeasures.some((measure) =>
    normalizeMeasureType(measure.measureType).includes("safeguard"),
  );
  const hasTradeRemedy = allMeasures.some((measure) =>
    isTradeRemedyType(normalizeMeasureType(measure.measureType)),
  );

  return (
    hasSafeguard &&
    hasTradeRemedy &&
    (normalizedType.includes("safeguard") || isTradeRemedyType(normalizedType))
  );
}

function isTradeRemedyType(normalizedType: string): boolean {
  return (
    normalizedType === "ad" ||
    normalizedType.includes("anti_dumping") ||
    normalizedType.includes("countervailing") ||
    normalizedType === "cvd"
  );
}

function normalizeMeasureType(measureType: string): string {
  return measureType.trim().toLowerCase().replaceAll(/[\s-]+/g, "_");
}

function canonicalSupplementaryUnit(
  unit: SupplementaryDutyUnit,
): SupplementaryQuantityKey {
  return unit === "kg" ? "kg_net" : unit;
}

function bpsDuty(customsValuePence: number, rateBps: number): number {
  return Math.round((customsValuePence * rateBps) / 10_000);
}

function measureId(measure: ActiveMeasure, index: number): string {
  return measure.id ?? `${measure.measureType}-${index + 1}`;
}

function unique<T>(values: readonly T[]): T[] {
  return [...new Set(values)];
}
