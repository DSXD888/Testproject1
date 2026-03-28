import { createHash } from "node:crypto";

export type CoverageQualificationResult = "pass" | "review" | "reject";

export interface CoverageCandidateInput {
  candidateId: string;
  hasPricing: boolean;
  hasLandedCost: boolean;
  passesNetGate: boolean;
  candidateValueShareBps: number | null;
}

export interface CoverageLotInput {
  lotId: string;
  qualificationResult: CoverageQualificationResult;
  isMandatory: boolean;
  lotAddressableValuePence: number | null;
  candidates: CoverageCandidateInput[];
}

export interface CoverageInput {
  lots: CoverageLotInput[];
}

export interface CoverageOutput {
  pricedCoverageBps: number;
  landedCostCoverageBps: number;
  netQualifiedCoverageBps: number;
  pricedValueCoverageBps: number;
  landedCostValueCoverageBps: number;
  netQualifiedValueCoverageBps: number;
  unresolvedMandatoryLots: number;
  decisionBasisHash: string;
}

export interface CoverageThresholdBand {
  tone: "green" | "amber" | "red";
  label: string;
}

export const AUTO_QUALIFICATION_VALUE_FLOOR_BPS = 5000;

export function describeCoverageThreshold(
  netQualifiedCoverageBps: number,
): CoverageThresholdBand {
  if (netQualifiedCoverageBps >= 7000) {
    return {
      tone: "green",
      label: "Strong coverage",
    };
  }

  if (netQualifiedCoverageBps >= 3000) {
    return {
      tone: "amber",
      label: "Partial coverage -- review recommended",
    };
  }

  return {
    tone: "red",
    label: "Low coverage -- manual review required",
  };
}

export function isAutoQualificationReady(output: CoverageOutput): boolean {
  return output.netQualifiedValueCoverageBps >= AUTO_QUALIFICATION_VALUE_FLOOR_BPS;
}

export function computeCoverageRollup(input: CoverageInput): CoverageOutput {
  const qualifyingLots = input.lots.filter((lot) => lot.qualificationResult !== "reject");
  const qualifyingLotCount = qualifyingLots.length;

  const pricedLotCount = qualifyingLots.filter((lot) =>
    lot.candidates.some((candidate) => candidate.hasPricing),
  ).length;
  const landedCostLotCount = qualifyingLots.filter((lot) =>
    lot.candidates.some((candidate) => candidate.hasLandedCost),
  ).length;
  const netQualifiedLotCount = qualifyingLots.filter((lot) =>
    lot.candidates.some((candidate) => candidate.passesNetGate),
  ).length;

  const unresolvedMandatoryLots = input.lots.filter((lot) => {
    if (!lot.isMandatory) {
      return false;
    }

    return !lot.candidates.some((candidate) => candidate.passesNetGate);
  }).length;

  const hasUnknownQualifyingValue = qualifyingLots.some(
    (lot) => lot.lotAddressableValuePence === null || lot.lotAddressableValuePence < 0,
  );
  const totalQualifyingValuePence = hasUnknownQualifyingValue
    ? 0
    : qualifyingLots.reduce(
        (sum, lot) => sum + (lot.lotAddressableValuePence ?? 0),
        0,
      );

  const pricedValuePence = qualifyingLots.reduce(
    (sum, lot) => sum + coveredValueForLot(lot, "pricing", hasUnknownQualifyingValue),
    0,
  );
  const landedCostValuePence = qualifyingLots.reduce(
    (sum, lot) => sum + coveredValueForLot(lot, "landed_cost", hasUnknownQualifyingValue),
    0,
  );
  const netQualifiedValuePence = qualifyingLots.reduce(
    (sum, lot) => sum + coveredValueForLot(lot, "net_gate", hasUnknownQualifyingValue),
    0,
  );

  return {
    pricedCoverageBps: toBasisPoints(pricedLotCount, qualifyingLotCount),
    landedCostCoverageBps: toBasisPoints(landedCostLotCount, qualifyingLotCount),
    netQualifiedCoverageBps: toBasisPoints(netQualifiedLotCount, qualifyingLotCount),
    pricedValueCoverageBps: toBasisPoints(pricedValuePence, totalQualifyingValuePence),
    landedCostValueCoverageBps: toBasisPoints(
      landedCostValuePence,
      totalQualifyingValuePence,
    ),
    netQualifiedValueCoverageBps: toBasisPoints(
      netQualifiedValuePence,
      totalQualifyingValuePence,
    ),
    unresolvedMandatoryLots,
    decisionBasisHash: hashCoverageInput(input),
  };
}

type CoverageLane = "pricing" | "landed_cost" | "net_gate";

function coveredValueForLot(
  lot: CoverageLotInput,
  lane: CoverageLane,
  hasUnknownQualifyingValue: boolean,
): number {
  if (hasUnknownQualifyingValue || lot.lotAddressableValuePence === null) {
    return 0;
  }

  const coveredShareBps = sumCoveredShareBps(lot.candidates, lane);
  return Math.round((lot.lotAddressableValuePence * coveredShareBps) / 10000);
}

function sumCoveredShareBps(
  candidates: CoverageCandidateInput[],
  lane: CoverageLane,
): number {
  let coveredShareBps = 0;
  for (const candidate of candidates) {
    const candidateCoversLane =
      lane === "pricing"
        ? candidate.hasPricing
        : lane === "landed_cost"
          ? candidate.hasLandedCost
          : candidate.passesNetGate;

    if (!candidateCoversLane || candidate.candidateValueShareBps === null) {
      continue;
    }

    coveredShareBps += candidate.candidateValueShareBps;
  }

  return Math.min(10000, coveredShareBps);
}

function toBasisPoints(numerator: number, denominator: number): number {
  if (denominator <= 0) {
    return 0;
  }

  return Math.round((numerator * 10000) / denominator);
}

function hashCoverageInput(input: CoverageInput): string {
  const normalizedLots = input.lots
    .map((lot) => ({
      lotId: lot.lotId,
      qualificationResult: lot.qualificationResult,
      isMandatory: lot.isMandatory,
      lotAddressableValuePence: lot.lotAddressableValuePence,
      candidates: [...lot.candidates]
        .sort((left, right) => left.candidateId.localeCompare(right.candidateId))
        .map((candidate) => ({
          candidateId: candidate.candidateId,
          hasPricing: candidate.hasPricing,
          hasLandedCost: candidate.hasLandedCost,
          passesNetGate: candidate.passesNetGate,
          candidateValueShareBps: candidate.candidateValueShareBps,
        })),
    }))
    .sort((left, right) => left.lotId.localeCompare(right.lotId));

  return createHash("sha256")
    .update(JSON.stringify({ lots: normalizedLots }))
    .digest("hex");
}
