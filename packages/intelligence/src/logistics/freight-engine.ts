import type { FreightEstimate, ModeSelection } from "./freight-types";
import type { FreightosAdapter } from "./freightos-skill-adapter";
import type { SearatesAdapter } from "./searates-skill-adapter";

export const FREIGHT_PROVIDER_DISAGREEMENT_THRESHOLD_BPS = 2500;

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
