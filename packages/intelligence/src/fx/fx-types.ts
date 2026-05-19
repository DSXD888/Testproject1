export type FxBasis =
  | "hmrc_customs_monthly"
  | "frankfurter_commercial_spot";

export type FxBaseCurrency = "USD" | "EUR" | "GBP" | "CNY";

export type FxQuoteCurrency = "GBP";

export interface FxSnapshot {
  basis: FxBasis;
  effectiveMonth?: string;
  effectiveDate?: string;
  baseCurrency: FxBaseCurrency;
  quoteCurrency: FxQuoteCurrency;
  rate: number;
  rateBps: number;
  sourceUrl: string;
  checkedAt: string;
  snapshotId: string;
}

export interface FxBasisDelta {
  customsSnapshot: FxSnapshot;
  commercialSnapshot: FxSnapshot;
  basisDeltaBps: number;
  computedAt: string;
}

export type FxRecalculationTrigger =
  | {
      kind: "monthly_rollover";
      previousMonth: string;
      newMonth: string;
    }
  | {
      kind: "historical_correction";
      affectedMonth: string;
      previousSnapshotId: string;
      newSnapshotId: string;
    }
  | {
      kind: "basis_delta_threshold_crossed";
      thresholdBps: number;
      observedBps: number;
    };
