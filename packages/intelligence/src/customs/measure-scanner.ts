import type { ExporterResolutionResult } from "./exporter-resolution";
import type { FreightEngineOutputs } from "../logistics/freight-engine";

export const MEASURE_INTERACTION_UNKNOWN = "MEASURE_INTERACTION_UNKNOWN" as const;
export const SUPPLEMENTARY_UNIT_MISSING = "SUPPLEMENTARY_UNIT_MISSING" as const;
export const VALUATION_MODE_CONFLICT = "VALUATION_MODE_CONFLICT" as const;

export const STANDARD_IMPORT_VAT_RATE_BPS = 2_000;

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

export type VatRateState = "standard" | "reduced" | "zero" | "unresolved";

export type ImportVatTreatment =
  | "standard_rate_default"
  | "reduced_rate_applied"
  | "zero_rate_applied"
  | "vatz_relief"
  | "vatr_relief"
  | "family_6_17_relief";

export type VatReliefClaimKind = "VATZ" | "VATR" | "family_6_17";

export interface VatReliefClaim {
  kind: VatReliefClaimKind;
  evidenceArtifactPaths: string[];
}

export interface VatEvidenceLeverage {
  leverageItem: string;
  marginDeltaBps: number;
  evidenceRequired: string[];
}

export interface VatResolution {
  vatRateState: VatRateState;
  vatRecoveryRateBps: number;
  importVatTreatment: ImportVatTreatment;
  evidenceLeverage: VatEvidenceLeverage[];
}

export type ConservativeDefaultParameter =
  | "origin_preference"
  | "exporter_rate"
  | "quota_allocation"
  | "vat_rate"
  | "incoterm"
  | "supplementary_units";

export interface EvidenceLeverageItem {
  parameter: ConservativeDefaultParameter;
  defaultApplied: string;
  evidenceRequired: string[];
  marginDeltaBps: number;
  unlockable: boolean;
  netMarginBaseBps: number;
  resolvedNetMarginBaseBps: number;
  snapshotId?: string;
}

export interface ConservativeDefaultSummary {
  default_applied: boolean;
  default_applied_value: string;
  evidence_required: string[];
  margin_delta_bps: number;
}

export interface ConfidenceBreakdown {
  unlockable_margin_bps: number;
  evidence_leverage: EvidenceLeverageItem[];
  active_defaults: string[];
  origin_preference?: ConservativeDefaultSummary;
  exporter_rate?: ConservativeDefaultSummary;
  quota_allocation?: ConservativeDefaultSummary;
  vat_rate?: ConservativeDefaultSummary;
  incoterm?: ConservativeDefaultSummary;
  supplementary_units?: ConservativeDefaultSummary;
}

export interface OriginPreferenceResolution {
  originProofAvailable: boolean;
  evidenceRequired?: string[];
}

export interface EvidenceLeverageMarginSnapshot {
  snapshotId?: string;
  netMarginBaseBps: number;
  resolvedNetMarginBaseBps: Partial<
    Record<ConservativeDefaultParameter, number>
  >;
}

export interface ComputeConfidenceBreakdownInput {
  exporterResolution: ExporterResolutionResult;
  vatResolution: VatResolution;
  measureScanResult: MeasureScanResult;
  marginSnapshot: EvidenceLeverageMarginSnapshot;
  originPreference?: OriginPreferenceResolution;
  freightOutput?: FreightEngineOutputs;
}

export type ValuationMode = "estimation" | "declaration";

export type CustomsValuationMethod =
  | "1_proxy"
  | "1_actual"
  | "2"
  | "3"
  | "4"
  | "5"
  | "6"
  | "unknown";

export interface ValuationOutput {
  valuationMode: ValuationMode;
  customsValuationMethod: CustomsValuationMethod;
  customsValuePence: number;
  conflictDetected: boolean;
  errorCode?: typeof VALUATION_MODE_CONFLICT;
  commissionsAssistsRoyaltiesStatus: "represented" | "unknown" | "absent";
}

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
    vatRateState?: VatRateState;
    vatRecoveryRateBps?: number;
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

export function resolveVat(opts: {
  uk10: string;
  measures: ActiveMeasure[];
  reliefClaims: VatReliefClaim[];
}): VatResolution {
  const importVatMeasure = opts.measures.find(isImportVatMeasure);
  const vatRecoveryRateBps = clampBps(
    importVatMeasure?.conditions?.vatRecoveryRateBps ?? 0,
  );
  const evidenceLeverage: VatEvidenceLeverage[] = [];

  for (const claim of opts.reliefClaims) {
    const missingEvidence = missingVatReliefEvidence(claim, opts);
    if (missingEvidence.length === 0) {
      return {
        vatRateState: reliefVatRateState(claim.kind),
        vatRecoveryRateBps,
        importVatTreatment: reliefImportVatTreatment(claim.kind),
        evidenceLeverage: [],
      };
    }

    evidenceLeverage.push({
      leverageItem: `vat_relief_${claim.kind.toLowerCase()}`,
      marginDeltaBps: reliefMarginDeltaBps(claim.kind),
      evidenceRequired: missingEvidence,
    });
  }

  const vatRateState = resolveVatRateState(importVatMeasure);

  if (vatRateState === "unresolved") {
    evidenceLeverage.push({
      leverageItem: "vat_rate_resolution",
      marginDeltaBps: STANDARD_IMPORT_VAT_RATE_BPS,
      evidenceRequired: ["hmrc_import_vat_measure"],
    });
  }

  return {
    vatRateState,
    vatRecoveryRateBps,
    importVatTreatment: importVatTreatmentForRateState(vatRateState),
    evidenceLeverage: uniqueByLeverageItem(evidenceLeverage),
  };
}

export function resolveValuation(opts: {
  proxyPricePence: number | null;
  declarationPricePence: number | null;
  commissionsPence?: number;
  assistsPence?: number;
  royaltiesPence?: number;
  resaleProceedsSharePence?: number;
  divergenceThresholdBps: number;
  customsValuationMethod?: CustomsValuationMethod;
}): ValuationOutput {
  const hasDeclaration = opts.declarationPricePence !== null;
  const hasProxy = opts.proxyPricePence !== null;
  const valuationMode: ValuationMode = hasDeclaration ? "declaration" : "estimation";
  const adjustmentsPence = valuationAdjustmentPence(opts);
  const baseValuePence =
    opts.declarationPricePence ?? opts.proxyPricePence ?? 0;
  const customsValuePence = baseValuePence + adjustmentsPence;
  const conflictDetected =
    hasDeclaration &&
    hasProxy &&
    valuationDivergenceBps(
      opts.proxyPricePence,
      opts.declarationPricePence,
    ) > opts.divergenceThresholdBps;

  return {
    valuationMode,
    customsValuationMethod: resolveCustomsValuationMethod(
      valuationMode,
      opts.customsValuationMethod,
    ),
    customsValuePence,
    conflictDetected,
    ...(conflictDetected ? { errorCode: VALUATION_MODE_CONFLICT } : {}),
    commissionsAssistsRoyaltiesStatus:
      commissionsAssistsRoyaltiesStatus(opts),
  };
}

export function computeConfidenceBreakdown(
  input: ComputeConfidenceBreakdownInput,
): ConfidenceBreakdown {
  const confidence: ConfidenceBreakdown = {
    unlockable_margin_bps: 0,
    evidence_leverage: [],
    active_defaults: [],
  };

  const recordDefault = (
    parameter: ConservativeDefaultParameter,
    defaultApplied: string,
    evidenceRequired: readonly string[],
    hardBlocker = false,
  ) => {
    const uniqueEvidenceRequired = unique(evidenceRequired).sort();
    const marginDeltaBps = evidenceLeverageMarginDeltaBps(
      parameter,
      input.marginSnapshot,
    );
    const summary: ConservativeDefaultSummary = {
      default_applied: true,
      default_applied_value: defaultApplied,
      evidence_required: uniqueEvidenceRequired,
      margin_delta_bps: marginDeltaBps,
    };

    confidence.active_defaults.push(`${parameter}:${defaultApplied}`);
    assignDefaultSummary(confidence, parameter, summary);

    if (marginDeltaBps > 0 || hardBlocker) {
      confidence.evidence_leverage.push({
        parameter,
        defaultApplied,
        evidenceRequired: uniqueEvidenceRequired,
        marginDeltaBps,
        unlockable: marginDeltaBps > 0,
        netMarginBaseBps: input.marginSnapshot.netMarginBaseBps,
        resolvedNetMarginBaseBps:
          input.marginSnapshot.resolvedNetMarginBaseBps[parameter] ??
          input.marginSnapshot.netMarginBaseBps,
        ...(input.marginSnapshot.snapshotId === undefined
          ? {}
          : { snapshotId: input.marginSnapshot.snapshotId }),
      });
    }
  };

  if (input.originPreference?.originProofAvailable === false) {
    recordDefault(
      "origin_preference",
      "MFN",
      input.originPreference.evidenceRequired ?? [
        "origin_certificate",
        "supplier_origin_declaration",
      ],
    );
  }

  if (
    input.exporterResolution.exporterRateBasis === "residual_default" &&
    (input.exporterResolution.errorCode !== undefined ||
      input.exporterResolution.missingEvidence.length > 0 ||
      input.exporterResolution.evidenceLeverage.length > 0)
  ) {
    recordDefault(
      "exporter_rate",
      "residual_default",
      input.exporterResolution.missingEvidence.length === 0
        ? ["named_exporter_proof"]
        : input.exporterResolution.missingEvidence,
    );
  }

  const quotaEvidenceRequired = input.measureScanResult.measureInteractionJson.conditions
    .filter((condition) => condition.selectedBranch === "out_of_quota")
    .flatMap((condition) => condition.evidenceRequired);
  if (quotaEvidenceRequired.length > 0) {
    recordDefault("quota_allocation", "out_of_quota", quotaEvidenceRequired);
  }

  if (
    input.vatResolution.importVatTreatment === "standard_rate_default" &&
    (input.vatResolution.vatRateState === "unresolved" ||
      input.vatResolution.evidenceLeverage.length > 0)
  ) {
    const vatEvidenceRequired = input.vatResolution.evidenceLeverage.flatMap(
      (entry) => entry.evidenceRequired,
    );
    recordDefault(
      "vat_rate",
      "standard_rate_default",
      vatEvidenceRequired.length === 0
        ? ["hmrc_import_vat_measure"]
        : vatEvidenceRequired,
    );
  }

  if (input.freightOutput?.missingLogisticsFacts.includes("incoterm") === true) {
    recordDefault("incoterm", "boundary_scenarios", ["incoterm"]);
  }

  if (input.measureScanResult.blockers.includes(SUPPLEMENTARY_UNIT_MISSING)) {
    const supplementaryEvidence =
      input.measureScanResult.supplementaryUnitsRequiredJson.map(
        (unit) => `supplementary_quantity:${unit}`,
      );
    recordDefault(
      "supplementary_units",
      "abstained",
      supplementaryEvidence.length === 0
        ? ["compatible_supplementary_unit_data"]
        : supplementaryEvidence,
      true,
    );
  }

  confidence.active_defaults = unique(confidence.active_defaults).sort();
  confidence.evidence_leverage = uniqueEvidenceLeverage(
    confidence.evidence_leverage,
  );
  confidence.unlockable_margin_bps = confidence.evidence_leverage.reduce(
    (sum, item) => sum + (item.unlockable ? item.marginDeltaBps : 0),
    0,
  );

  return confidence;
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
    additionalCodesRequirementsJson: conditions.flatMap((condition) => {
      if (condition.additionalCode !== undefined) {
        return [
          {
            code: condition.additionalCode,
            satisfiability: condition.satisfiability ?? "unknown",
            evidenceRequired: condition.evidenceRequired,
          },
        ];
      }

      if (condition.quotaCode !== undefined) {
        return [
          {
            code: condition.quotaCode,
            satisfiability:
              condition.selectedBranch === "in_quota" ? "satisfied" : "unknown",
            evidenceRequired: condition.evidenceRequired,
          },
        ];
      }

      return [];
    }),
  };
}

function evidenceLeverageMarginDeltaBps(
  parameter: ConservativeDefaultParameter,
  marginSnapshot: EvidenceLeverageMarginSnapshot,
): number {
  const resolvedNetMarginBps =
    marginSnapshot.resolvedNetMarginBaseBps[parameter];
  if (resolvedNetMarginBps === undefined) {
    return 0;
  }

  return Math.max(0, resolvedNetMarginBps - marginSnapshot.netMarginBaseBps);
}

function assignDefaultSummary(
  confidence: ConfidenceBreakdown,
  parameter: ConservativeDefaultParameter,
  summary: ConservativeDefaultSummary,
): void {
  switch (parameter) {
    case "origin_preference":
      confidence.origin_preference = summary;
      return;
    case "exporter_rate":
      confidence.exporter_rate = summary;
      return;
    case "quota_allocation":
      confidence.quota_allocation = summary;
      return;
    case "vat_rate":
      confidence.vat_rate = summary;
      return;
    case "incoterm":
      confidence.incoterm = summary;
      return;
    case "supplementary_units":
      confidence.supplementary_units = summary;
      return;
  }
}

function uniqueEvidenceLeverage(
  items: readonly EvidenceLeverageItem[],
): EvidenceLeverageItem[] {
  const byParameter = new Map<ConservativeDefaultParameter, EvidenceLeverageItem>();
  for (const item of items) {
    byParameter.set(item.parameter, item);
  }
  return [...byParameter.values()].sort((left, right) =>
    left.parameter.localeCompare(right.parameter),
  );
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

function isImportVatMeasure(measure: ActiveMeasure): boolean {
  const normalizedType = normalizeMeasureType(measure.measureType);
  return (
    normalizedType.includes("import_vat") ||
    normalizedType.includes("vat") ||
    normalizedType.includes("value_added_tax")
  );
}

function missingVatReliefEvidence(
  claim: VatReliefClaim,
  opts: {
    uk10: string;
    measures: ActiveMeasure[];
  },
): string[] {
  return [
    ...(vatCommodityPrerequisiteSatisfied(claim.kind, opts)
      ? []
      : ["commodity_prerequisite"]),
    ...(claim.evidenceArtifactPaths.length > 0
      ? []
      : ["documentary_relief_evidence"]),
  ];
}

function vatCommodityPrerequisiteSatisfied(
  kind: VatReliefClaimKind,
  opts: {
    uk10: string;
    measures: ActiveMeasure[];
  },
): boolean {
  const normalizedKind = normalizeVatReliefKind(kind);
  return opts.measures.some((measure) => {
    const measureText = [
      measure.measureType,
      measure.conditions?.additionalCode ?? "",
      measure.id ?? "",
    ]
      .join(" ")
      .toLowerCase();

    if (normalizedKind === "family_6_17") {
      return (
        measureText.includes("6/17") ||
        measureText.includes("6_17") ||
        measureText.includes("de 6") ||
        opts.uk10.startsWith("4819")
      );
    }

    return measureText.includes(normalizedKind.toLowerCase());
  });
}

function normalizeVatReliefKind(kind: VatReliefClaimKind): string {
  return kind === "family_6_17" ? "family_6_17" : kind;
}

function reliefVatRateState(kind: VatReliefClaimKind): VatRateState {
  return kind === "VATR" ? "reduced" : "zero";
}

function reliefImportVatTreatment(kind: VatReliefClaimKind): ImportVatTreatment {
  switch (kind) {
    case "VATZ":
      return "vatz_relief";
    case "VATR":
      return "vatr_relief";
    case "family_6_17":
      return "family_6_17_relief";
  }
}

function reliefMarginDeltaBps(kind: VatReliefClaimKind): number {
  return kind === "VATR" ? 1_500 : STANDARD_IMPORT_VAT_RATE_BPS;
}

function resolveVatRateState(measure: ActiveMeasure | undefined): VatRateState {
  if (measure === undefined) {
    return "unresolved";
  }

  if (measure.conditions?.vatRateState !== undefined) {
    return measure.conditions.vatRateState;
  }

  if (measure.dutyExpression.kind !== "ad_valorem") {
    return "unresolved";
  }

  if (measure.dutyExpression.rateBps === 0) {
    return "zero";
  }

  if (measure.dutyExpression.rateBps < STANDARD_IMPORT_VAT_RATE_BPS) {
    return "reduced";
  }

  if (measure.dutyExpression.rateBps === STANDARD_IMPORT_VAT_RATE_BPS) {
    return "standard";
  }

  return "unresolved";
}

function importVatTreatmentForRateState(
  vatRateState: VatRateState,
): ImportVatTreatment {
  switch (vatRateState) {
    case "standard":
    case "unresolved":
      return "standard_rate_default";
    case "reduced":
      return "reduced_rate_applied";
    case "zero":
      return "zero_rate_applied";
  }
}

function uniqueByLeverageItem(
  entries: readonly VatEvidenceLeverage[],
): VatEvidenceLeverage[] {
  const byItem = new Map<string, VatEvidenceLeverage>();
  for (const entry of entries) {
    byItem.set(entry.leverageItem, entry);
  }
  return [...byItem.values()];
}

function valuationAdjustmentPence(opts: {
  commissionsPence?: number;
  assistsPence?: number;
  royaltiesPence?: number;
  resaleProceedsSharePence?: number;
}): number {
  return (
    (opts.commissionsPence ?? 0) +
    (opts.assistsPence ?? 0) +
    (opts.royaltiesPence ?? 0) +
    (opts.resaleProceedsSharePence ?? 0)
  );
}

function valuationDivergenceBps(
  proxyPricePence: number | null,
  declarationPricePence: number | null,
): number {
  if (proxyPricePence === null || declarationPricePence === null) {
    return 0;
  }

  const denominator = Math.max(Math.abs(declarationPricePence), 1);
  return Math.round(
    (Math.abs(proxyPricePence - declarationPricePence) * 10_000) / denominator,
  );
}

function resolveCustomsValuationMethod(
  valuationMode: ValuationMode,
  overrideMethod: CustomsValuationMethod | undefined,
): CustomsValuationMethod {
  if (
    valuationMode === "declaration" &&
    overrideMethod !== undefined &&
    overrideMethod !== "1_proxy" &&
    overrideMethod !== "1_actual"
  ) {
    return overrideMethod;
  }

  return valuationMode === "estimation" ? "1_proxy" : "1_actual";
}

function commissionsAssistsRoyaltiesStatus(opts: {
  commissionsPence?: number;
  assistsPence?: number;
  royaltiesPence?: number;
  resaleProceedsSharePence?: number;
}): ValuationOutput["commissionsAssistsRoyaltiesStatus"] {
  const values = [
    opts.commissionsPence,
    opts.assistsPence,
    opts.royaltiesPence,
    opts.resaleProceedsSharePence,
  ];

  if (values.every((value) => value === undefined)) {
    return "unknown";
  }

  return values.some((value) => (value ?? 0) > 0) ? "represented" : "absent";
}

function clampBps(value: number): number {
  return Math.min(10_000, Math.max(0, Math.round(value)));
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
