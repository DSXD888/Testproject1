import { selectMode } from "./mode-selector";
import type {
  FreightEstimate,
  ModeSelection,
  ModeSelectionFacts,
  PackagingState,
  ShippingMode,
} from "./freight-types";
import type { FreightosAdapter } from "./freightos-skill-adapter";
import type { SearatesAdapter } from "./searates-skill-adapter";

export const FREIGHT_PROVIDER_DISAGREEMENT_THRESHOLD_BPS = 2500;
export const FREIGHT_DIMENSIONAL_DATA_MISSING =
  "FREIGHT_DIMENSIONAL_DATA_MISSING" as const;

export type DualProviderOutcome =
  | {
      kind: "agreed";
      selected: FreightEstimate;
      otherProvider: FreightEstimate;
    }
  | {
      kind: "disagreement";
      selected: FreightEstimate;
      otherProvider: FreightEstimate;
      deltaBps: number;
      warning: "FREIGHT_MODE_LOW_CONFIDENCE";
    }
  | {
      kind: "single_provider";
      selected: FreightEstimate;
      missingProvider: "freightos" | "searates";
    }
  | {
      kind: "dual_failure";
      failure: "FREIGHT_ESTIMATE_FAILED";
      manualEstimateRequired: true;
    };

export interface DualProviderEstimateOptions {
  freightos: FreightosAdapter;
  searates: SearatesAdapter;
  mode: ModeSelection["mode"];
  originPort: string;
  destinationPort: string;
  weightKg: number;
  volumeM3: number;
  freightosCassettePath?: string;
  searatesCassettePath?: string;
}

export type FreightIncoterm = "EXW" | "FOB" | "CIF" | "DDP" | "unknown";
export type BoundaryIncoterm = Exclude<FreightIncoterm, "unknown">;
export type BoundaryPackagingState = Exclude<PackagingState, "unknown">;
export type FreightEngineOutcomeKind =
  | DualProviderOutcome["kind"]
  | "manual_required";

export type FreightEngineFacts = Omit<
  ModeSelectionFacts,
  "weightKg" | "volumeM3" | "packagingState"
> & {
  weightKg: number | null;
  volumeM3: number | null;
  packagingState: PackagingState;
};

export interface FreightEngineInputs {
  facts: FreightEngineFacts;
  incoterm: FreightIncoterm;
  originPort: string;
  destinationPort: string;
  freightosCassettePath?: string;
  searatesCassettePath?: string;
}

export interface FreightBoundaryScenario {
  scenario: string;
  incoterm: BoundaryIncoterm;
  packagingState: BoundaryPackagingState;
  selectedMode: ShippingMode | null;
  estimate: FreightEstimate | null;
  outcomeKind: FreightEngineOutcomeKind;
  warningsAndBlockers: string[];
  missingLogisticsFacts: string[];
}

export interface FreightEngineOutputs {
  selectedMode: ShippingMode | null;
  selectedEstimate: FreightEstimate | null;
  selectedIncoterm: BoundaryIncoterm | null;
  selectedPackagingState: BoundaryPackagingState | null;
  boundaryScenariosRun: FreightBoundaryScenario[];
  outcomeKind: FreightEngineOutcomeKind;
  warningsAndBlockers: string[];
  manualEstimateRequired: boolean;
  missingLogisticsFacts: string[];
  unlockableMarginNote?: string;
}

export class FreightEngine {
  constructor(
    private readonly freightos: FreightosAdapter,
    private readonly searates: SearatesAdapter,
  ) {}

  async estimate(inputs: FreightEngineInputs): Promise<FreightEngineOutputs> {
    const incoterms = incotermScenarios(inputs.incoterm);
    const packagingStates = packagingScenarios(inputs.facts.packagingState);
    const missingLogisticsFacts = missingFacts(inputs);
    const dimensionFactsMissing =
      missingLogisticsFacts.includes("weight_kg") ||
      missingLogisticsFacts.includes("volume_m3");

    if (dimensionFactsMissing) {
      const warningsAndBlockers = [FREIGHT_DIMENSIONAL_DATA_MISSING];
      return {
        selectedMode: null,
        selectedEstimate: null,
        selectedIncoterm: null,
        selectedPackagingState: null,
        boundaryScenariosRun: manualBoundaryScenarios(
          incoterms,
          packagingStates,
          warningsAndBlockers,
          missingLogisticsFacts,
        ),
        outcomeKind: "manual_required",
        warningsAndBlockers,
        manualEstimateRequired: true,
        missingLogisticsFacts,
        unlockableMarginNote: unlockableMarginNote(missingLogisticsFacts),
      };
    }

    const boundaryScenariosRun: FreightBoundaryScenario[] = [];
    for (const incoterm of incoterms) {
      for (const packagingState of packagingStates) {
        boundaryScenariosRun.push(
          await this.estimateBoundaryScenario(inputs, incoterm, packagingState),
        );
      }
    }

    const selectedScenario = selectHighestCostScenario(boundaryScenariosRun);
    const warningsAndBlockers = uniqueStrings(
      boundaryScenariosRun.flatMap((scenario) => scenario.warningsAndBlockers),
    );

    return {
      selectedMode: selectedScenario?.selectedMode ?? null,
      selectedEstimate: selectedScenario?.estimate ?? null,
      selectedIncoterm: selectedScenario?.incoterm ?? null,
      selectedPackagingState: selectedScenario?.packagingState ?? null,
      boundaryScenariosRun,
      outcomeKind:
        selectedScenario?.outcomeKind ??
        boundaryScenariosRun[0]?.outcomeKind ??
        "manual_required",
      warningsAndBlockers,
      manualEstimateRequired: selectedScenario === null,
      missingLogisticsFacts,
      unlockableMarginNote: unlockableMarginNote(missingLogisticsFacts),
    };
  }

  private async estimateBoundaryScenario(
    inputs: FreightEngineInputs,
    incoterm: BoundaryIncoterm,
    packagingState: BoundaryPackagingState,
  ): Promise<FreightBoundaryScenario> {
    if (inputs.facts.weightKg === null || inputs.facts.volumeM3 === null) {
      throw new Error("dimension facts must be present before scenario estimation");
    }

    const facts: ModeSelectionFacts = {
      ...inputs.facts,
      weightKg: inputs.facts.weightKg,
      volumeM3: inputs.facts.volumeM3,
      packagingState,
    };
    const modeSelection = selectMode(facts);
    const request = {
      mode: modeSelection.mode,
      originPort: inputs.originPort,
      destinationPort: inputs.destinationPort,
      weightKg: facts.weightKg,
      volumeM3: facts.volumeM3,
      incoterm,
      packagingState,
      scenario: scenarioName(incoterm, packagingState),
    };

    const [freightosEstimate, searatesEstimate] = await Promise.all([
      this.freightos.estimate({
        ...request,
        cassettePath: inputs.freightosCassettePath,
      }),
      this.searates.estimate({
        ...request,
        cassettePath: inputs.searatesCassettePath,
      }),
    ]);
    const outcome = reconcile(freightosEstimate, searatesEstimate);
    return scenarioFromOutcome(
      scenarioName(incoterm, packagingState),
      incoterm,
      packagingState,
      modeSelection.mode,
      outcome,
    );
  }
}

export async function estimateWithDualProviders(
  opts: DualProviderEstimateOptions,
): Promise<DualProviderOutcome> {
  const sharedRequest = {
    mode: opts.mode,
    originPort: opts.originPort,
    destinationPort: opts.destinationPort,
    weightKg: opts.weightKg,
    volumeM3: opts.volumeM3,
  };

  const [freightos, searates] = await Promise.all([
    opts.freightos.estimate({
      ...sharedRequest,
      cassettePath: opts.freightosCassettePath,
    }),
    opts.searates.estimate({
      ...sharedRequest,
      cassettePath: opts.searatesCassettePath,
    }),
  ]);

  return reconcile(freightos, searates);
}

export function reconcile(
  freightos: FreightEstimate | null,
  searates: FreightEstimate | null,
): DualProviderOutcome {
  const freightosOk = hasBaseQuote(freightos);
  const searatesOk = hasBaseQuote(searates);

  if (!freightosOk && !searatesOk) {
    return {
      kind: "dual_failure",
      failure: "FREIGHT_ESTIMATE_FAILED",
      manualEstimateRequired: true,
    };
  }

  if (freightosOk && !searatesOk) {
    return {
      kind: "single_provider",
      selected: freightos,
      missingProvider: "searates",
    };
  }

  if (!freightosOk && searatesOk) {
    return {
      kind: "single_provider",
      selected: searates,
      missingProvider: "freightos",
    };
  }

  if (!hasBaseQuote(freightos) || !hasBaseQuote(searates)) {
    throw new Error("dual-provider reconciliation reached an unreachable state");
  }

  const freightosBase = freightos.baseGbpPence;
  const searatesBase = searates.baseGbpPence;
  const max = Math.max(freightosBase, searatesBase);
  const min = Math.min(freightosBase, searatesBase);
  const selected = max === freightosBase ? freightos : searates;
  const otherProvider = max === freightosBase ? searates : freightos;
  const deltaBps =
    min === 0
      ? max === 0
        ? 0
        : Number.MAX_SAFE_INTEGER
      : Math.round(((max - min) / min) * 10_000);

  if (deltaBps > FREIGHT_PROVIDER_DISAGREEMENT_THRESHOLD_BPS) {
    return {
      kind: "disagreement",
      selected,
      otherProvider,
      deltaBps,
      warning: "FREIGHT_MODE_LOW_CONFIDENCE",
    };
  }

  return {
    kind: "agreed",
    selected,
    otherProvider,
  };
}

function hasBaseQuote(
  estimate: FreightEstimate | null,
): estimate is FreightEstimate & { baseGbpPence: number } {
  return estimate !== null && estimate.baseGbpPence !== null;
}

function incotermScenarios(incoterm: FreightIncoterm): BoundaryIncoterm[] {
  return incoterm === "unknown" ? ["EXW", "FOB", "CIF"] : [incoterm];
}

function packagingScenarios(
  packagingState: PackagingState,
): BoundaryPackagingState[] {
  return packagingState === "unknown"
    ? ["palletized", "loose_cartons"]
    : [packagingState];
}

function missingFacts(inputs: FreightEngineInputs): string[] {
  const facts: string[] = [];
  if (inputs.incoterm === "unknown") {
    facts.push("incoterm");
  }
  if (inputs.facts.packagingState === "unknown") {
    facts.push("packaging_state");
  }
  if (inputs.facts.weightKg === null) {
    facts.push("weight_kg");
  }
  if (inputs.facts.volumeM3 === null) {
    facts.push("volume_m3");
  }
  if (inputs.facts.productType.trim() === "") {
    facts.push("product_type");
  }
  return facts;
}

function manualBoundaryScenarios(
  incoterms: BoundaryIncoterm[],
  packagingStates: BoundaryPackagingState[],
  warningsAndBlockers: string[],
  missingLogisticsFacts: string[],
): FreightBoundaryScenario[] {
  return incoterms.flatMap((incoterm) =>
    packagingStates.map((packagingState) => ({
      scenario: scenarioName(incoterm, packagingState),
      incoterm,
      packagingState,
      selectedMode: null,
      estimate: null,
      outcomeKind: "manual_required" as const,
      warningsAndBlockers,
      missingLogisticsFacts,
    })),
  );
}

function scenarioFromOutcome(
  scenario: string,
  incoterm: BoundaryIncoterm,
  packagingState: BoundaryPackagingState,
  selectedMode: ShippingMode,
  outcome: DualProviderOutcome,
): FreightBoundaryScenario {
  if (outcome.kind === "dual_failure") {
    return {
      scenario,
      incoterm,
      packagingState,
      selectedMode,
      estimate: null,
      outcomeKind: outcome.kind,
      warningsAndBlockers: [outcome.failure],
      missingLogisticsFacts: [],
    };
  }

  if (outcome.kind === "disagreement") {
    return {
      scenario,
      incoterm,
      packagingState,
      selectedMode,
      estimate: outcome.selected,
      outcomeKind: outcome.kind,
      warningsAndBlockers: [outcome.warning],
      missingLogisticsFacts: [],
    };
  }

  return {
    scenario,
    incoterm,
    packagingState,
    selectedMode,
    estimate: outcome.selected,
    outcomeKind: outcome.kind,
    warningsAndBlockers: [],
    missingLogisticsFacts: [],
  };
}

function selectHighestCostScenario(
  scenarios: FreightBoundaryScenario[],
): FreightBoundaryScenario | null {
  let selected: FreightBoundaryScenario | null = null;
  for (const scenario of scenarios) {
    if (scenario.estimate?.baseGbpPence === null || scenario.estimate === null) {
      continue;
    }
    if (
      selected === null ||
      scenario.estimate.baseGbpPence > selected.estimate!.baseGbpPence!
    ) {
      selected = scenario;
    }
  }
  return selected;
}

function uniqueStrings(values: string[]): string[] {
  return [...new Set(values)];
}

function unlockableMarginNote(
  missingLogisticsFacts: string[],
): string | undefined {
  if (missingLogisticsFacts.length === 0) {
    return undefined;
  }
  return `Clarify ${missingLogisticsFacts.join(
    ", ",
  )} to replace conservative freight boundary scenarios.`;
}

function scenarioName(
  incoterm: BoundaryIncoterm,
  packagingState: BoundaryPackagingState,
): string {
  return `${incoterm}:${packagingState}`;
}
