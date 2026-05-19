import type { FrankfurterFetcher } from "./frankfurter-commercial-rates";
import type { HmrcMonthlyFetcher } from "./hmrc-exchange-rates";
import type {
  FxBasisDelta,
  FxRecalculationTrigger,
  FxSnapshot,
} from "./fx-types";

export const FX_DELTA_RECALC_THRESHOLD_BPS = 150;
export const FX_BASIS_DELTA_ROUNDING_RULE = "round-half-up";

export interface NetMarginBand {
  low: number;
  base: number;
  high: number;
}

export interface DualBasisNetMarginBand {
  underCustoms: NetMarginBand;
  underCommercial: NetMarginBand;
}

export interface LandedCostFxProjection {
  table: "landed_cost_estimates";
  syncAuthority: "l4_push_owned_reseed";
  schemaMutationRequired: false;
  values: {
    customs_fx_rate: number;
    commercial_fx_rate: number;
    fx_basis_delta_bps: number;
    fx_rate_checked_at: string;
  };
}

export interface FxProviderPolicyEvidence {
  providers: Array<{
    id: "hmrc_monthly" | "frankfurter";
    role: "customs_monthly" | "commercial_spot";
    auth: "none";
    procurement: "public_free_no_key";
  }>;
  paidCommercialFxProcurementRequired: false;
}

export interface FxEngine {
  rateForCustoms(
    baseCurrency: string,
    effectiveMonth: string,
  ): Promise<FxSnapshot>;

  rateForCommercial(baseCurrency: string): Promise<FxSnapshot>;

  computeBasisDelta(customs: FxSnapshot, commercial: FxSnapshot): FxBasisDelta;

  evaluateRecalculation(
    delta: FxBasisDelta,
    thresholdBps?: number,
  ): FxRecalculationTrigger | null;

  netMarginBandUnderBothBases(
    netMarginBase: NetMarginBand,
    customsRate: number,
    commercialRate: number,
  ): DualBasisNetMarginBand;
}

export function createFxEngine(opts: {
  hmrc: HmrcMonthlyFetcher;
  frankfurter: FrankfurterFetcher;
  now?: () => Date;
}): FxEngine {
  const now = opts.now ?? (() => new Date());

  return {
    rateForCustoms(baseCurrency, effectiveMonth) {
      return opts.hmrc.fetchSnapshot({
        baseCurrency: toSupportedBaseCurrency(baseCurrency),
        effectiveMonth,
      });
    },

    rateForCommercial(baseCurrency) {
      return opts.frankfurter.fetchSpot({
        baseCurrency: toSupportedBaseCurrency(baseCurrency),
      });
    },

    computeBasisDelta(customs, commercial) {
      return computeFxBasisDelta(customs, commercial, now().toISOString());
    },

    evaluateRecalculation(delta, thresholdBps = FX_DELTA_RECALC_THRESHOLD_BPS) {
      return evaluateFxRecalculation(delta, thresholdBps);
    },

    netMarginBandUnderBothBases,
  };
}

export function computeFxBasisDelta(
  customs: FxSnapshot,
  commercial: FxSnapshot,
  computedAt: string,
): FxBasisDelta {
  assertCompatibleSnapshots(customs, commercial);
  const rawDeltaBps = ((commercial.rate - customs.rate) / customs.rate) * 10000;
  return {
    customsSnapshot: customs,
    commercialSnapshot: commercial,
    basisDeltaBps: roundHalfUp(rawDeltaBps),
    computedAt,
  };
}

export function evaluateFxRecalculation(
  delta: FxBasisDelta,
  thresholdBps = FX_DELTA_RECALC_THRESHOLD_BPS,
): FxRecalculationTrigger | null {
  if (Math.abs(delta.basisDeltaBps) < thresholdBps) {
    return null;
  }

  return {
    kind: "basis_delta_threshold_crossed",
    thresholdBps,
    observedBps: delta.basisDeltaBps,
  };
}

export function netMarginBandUnderBothBases(
  netMarginBase: NetMarginBand,
  customsRate: number,
  commercialRate: number,
): DualBasisNetMarginBand {
  const basisDeltaBps = roundHalfUp(
    ((commercialRate - customsRate) / customsRate) * 10000,
  );
  return {
    underCustoms: { ...netMarginBase },
    underCommercial: {
      low: netMarginBase.low - basisDeltaBps,
      base: netMarginBase.base - basisDeltaBps,
      high: netMarginBase.high - basisDeltaBps,
    },
  };
}

export function projectFxSnapshotsToLandedCostEstimate(opts: {
  customsSnapshot: FxSnapshot;
  commercialSnapshot: FxSnapshot;
  fxRateCheckedAt: string;
}): LandedCostFxProjection {
  const delta = computeFxBasisDelta(
    opts.customsSnapshot,
    opts.commercialSnapshot,
    opts.fxRateCheckedAt,
  );

  return {
    table: "landed_cost_estimates",
    syncAuthority: "l4_push_owned_reseed",
    schemaMutationRequired: false,
    values: {
      customs_fx_rate: opts.customsSnapshot.rate,
      commercial_fx_rate: opts.commercialSnapshot.rate,
      fx_basis_delta_bps: delta.basisDeltaBps,
      fx_rate_checked_at: opts.fxRateCheckedAt,
    },
  };
}

export function fxProviderPolicyEvidence(): FxProviderPolicyEvidence {
  return {
    providers: [
      {
        id: "hmrc_monthly",
        role: "customs_monthly",
        auth: "none",
        procurement: "public_free_no_key",
      },
      {
        id: "frankfurter",
        role: "commercial_spot",
        auth: "none",
        procurement: "public_free_no_key",
      },
    ],
    paidCommercialFxProcurementRequired: false,
  };
}

function assertCompatibleSnapshots(customs: FxSnapshot, commercial: FxSnapshot): void {
  if (customs.basis !== "hmrc_customs_monthly") {
    throw new Error("Customs FX snapshot must use the HMRC monthly basis");
  }
  if (commercial.basis !== "frankfurter_commercial_spot") {
    throw new Error("Commercial FX snapshot must use the Frankfurter spot basis");
  }
  if (
    customs.baseCurrency !== commercial.baseCurrency ||
    customs.quoteCurrency !== commercial.quoteCurrency
  ) {
    throw new Error("FX basis delta requires matching currency pair snapshots");
  }
  if (customs.rate <= 0 || commercial.rate <= 0) {
    throw new Error("FX basis delta requires positive rates");
  }
}

function toSupportedBaseCurrency(baseCurrency: string): "USD" | "EUR" | "CNY" {
  if (baseCurrency === "USD" || baseCurrency === "EUR" || baseCurrency === "CNY") {
    return baseCurrency;
  }

  throw new Error(`Unsupported FX base currency ${baseCurrency}`);
}

function roundHalfUp(value: number): number {
  if (!Number.isFinite(value)) {
    throw new Error("Cannot round a non-finite FX basis delta");
  }

  return value >= 0 ? Math.floor(value + 0.5) : Math.ceil(value - 0.5);
}
