import type {
  HardLock,
  ModeSelection,
  ModeSelectionFacts,
  ShippingMode,
} from "./freight-types";

const FCL_VOLUME_M3 = 15;
const EXPRESS_MAX_WEIGHT_KG = 70;
const EXPRESS_MAX_VOLUME_M3 = 0.5;
const HIGH_VALUE_GBP_PENCE = 5_000_000;

export function selectMode(facts: ModeSelectionFacts): ModeSelection {
  assertNonNegativeFinite(facts.weightKg, "weightKg");
  assertNonNegativeFinite(facts.volumeM3, "volumeM3");
  assertNonNegativeFinite(facts.valueGbpPence, "valueGbpPence");

  const hardLocksTriggered = hardLocksFor(facts);
  const fallbackBoundaryUsed = facts.packagingState === "unknown";
  const reasonParts = [
    `urgency=${facts.urgency}`,
    `weight=${facts.weightKg}kg`,
    `volume=${facts.volumeM3}m3`,
    `value=${facts.valueGbpPence}p`,
    `product=${facts.productType}`,
    `packaging=${facts.packagingState}`,
  ];

  let mode: ShippingMode;
  if (fallbackBoundaryUsed) {
    mode = highestCostFeasibleMode(facts);
    reasonParts.push("unknown packaging uses highest-cost feasible boundary");
  } else {
    mode = selectModeWithoutPackagingFallback(facts);
  }

  if (hardLocksTriggered.length > 0) {
    reasonParts.push(
      `hard locks: ${hardLocksTriggered.map((lock) => lock.kind).join(",")}`,
    );
  }

  return {
    mode,
    confidenceBps: fallbackBoundaryUsed ? 6500 : confidenceFor(mode, hardLocksTriggered),
    hardLocksTriggered,
    reason: reasonParts.join("; "),
    fallbackBoundaryUsed,
  };
}

function selectModeWithoutPackagingFallback(
  facts: ModeSelectionFacts,
): ShippingMode {
  if (facts.hazmat || isAirRestrictedBattery(facts.batteries)) {
    return seaModeFor(facts);
  }

  if (facts.reefer) {
    return facts.volumeM3 <= 2 && facts.urgency !== "standard"
      ? "air_general"
      : "sea_fcl";
  }

  if (facts.fragile && facts.urgency === "critical") {
    return "air_general";
  }

  if (facts.oversized) {
    return facts.urgency === "standard" ? seaModeFor(facts) : "air_general";
  }

  if (
    facts.volumeM3 < EXPRESS_MAX_VOLUME_M3 &&
    facts.weightKg <= EXPRESS_MAX_WEIGHT_KG &&
    facts.urgency === "critical"
  ) {
    return "air_express";
  }

  if (
    (facts.volumeM3 < 5 && facts.urgency === "expedited") ||
    (isCareSensitiveProduct(facts.productType) && facts.urgency === "critical") ||
    (facts.valueGbpPence >= HIGH_VALUE_GBP_PENCE &&
      facts.urgency !== "standard" &&
      facts.volumeM3 < FCL_VOLUME_M3)
  ) {
    return "air_general";
  }

  if (
    facts.valueGbpPence >= HIGH_VALUE_GBP_PENCE &&
    facts.volumeM3 < FCL_VOLUME_M3
  ) {
    return "multimodal";
  }

  return seaModeFor(facts);
}

function highestCostFeasibleMode(facts: ModeSelectionFacts): ShippingMode {
  const candidates: ShippingMode[] = [
    "air_express",
    "air_general",
    "multimodal",
    "sea_fcl",
    "sea_lcl",
  ];
  return (
    candidates.find((candidate) => modeIsFeasible(candidate, facts)) ??
    seaModeFor(facts)
  );
}

function modeIsFeasible(mode: ShippingMode, facts: ModeSelectionFacts): boolean {
  if (mode === "air_express") {
    return (
      !facts.hazmat &&
      !facts.oversized &&
      !facts.reefer &&
      !isAirRestrictedBattery(facts.batteries) &&
      facts.weightKg <= EXPRESS_MAX_WEIGHT_KG &&
      facts.volumeM3 < EXPRESS_MAX_VOLUME_M3
    );
  }

  if (mode === "air_general") {
    return (
      !facts.hazmat &&
      !isAirRestrictedBattery(facts.batteries) &&
      (!facts.reefer || facts.volumeM3 <= 2)
    );
  }

  if (mode === "multimodal") {
    return !facts.hazmat && !facts.reefer;
  }

  return true;
}

function hardLocksFor(facts: ModeSelectionFacts): HardLock[] {
  const locks: HardLock[] = [];

  if (facts.hazmat) {
    locks.push({
      kind: "hazmat",
      details: "hazmat goods exclude air modes unless operator evidence proves air acceptance",
    });
  }

  if (isAirRestrictedBattery(facts.batteries)) {
    locks.push({
      kind: "battery",
      details: `battery state ${facts.batteries} restricts air modes`,
    });
  }

  if (facts.reefer) {
    locks.push({
      kind: "reefer",
      details: "temperature-controlled freight requires reefer-capable mode",
    });
  }

  if (facts.oversized) {
    locks.push({
      kind: "oversized",
      details: "oversized freight excludes air express",
    });
  }

  if (facts.fragile && facts.urgency === "critical") {
    locks.push({
      kind: "fragile",
      details: "critical fragile freight requires controlled rapid handling",
    });
  }

  return locks;
}

function seaModeFor(facts: ModeSelectionFacts): ShippingMode {
  return facts.volumeM3 >= FCL_VOLUME_M3 ? "sea_fcl" : "sea_lcl";
}

function isAirRestrictedBattery(
  batteries: ModeSelectionFacts["batteries"],
): boolean {
  return batteries === "loose" || batteries === "lithium_unknown";
}

function isCareSensitiveProduct(productType: string): boolean {
  return /\b(glass|ceramic|diagnostic|medical|precision|electronics)\b/i.test(
    productType,
  );
}

function confidenceFor(mode: ShippingMode, hardLocks: HardLock[]): number {
  if (hardLocks.length === 0) {
    return mode === "multimodal" ? 7600 : 8400;
  }
  return 7900;
}

function assertNonNegativeFinite(value: number, field: string): void {
  if (!Number.isFinite(value) || value < 0) {
    throw new RangeError(`${field} must be a non-negative finite number`);
  }
}
