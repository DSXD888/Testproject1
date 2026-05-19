export const TRADE_REMEDY_WATCH_STALE = "TRADE_REMEDY_WATCH_STALE";
export const TARIFF_STRUCTURE_CHANGED = "TARIFF_STRUCTURE_CHANGED";
export const TRA_INVESTIGATION_OPEN = "TRA_INVESTIGATION_OPEN";

const MS_PER_DAY = 24 * 60 * 60 * 1000;

export type FreshnessVector = "structure" | "measures" | "remedies" | "fx";

export type TradeRemedyWatchState =
  | "stable"
  | "investigation_open"
  | "provisional_measure_active"
  | "definitive_measure_changed"
  | "stale";

export interface FreshnessVectors {
  tariffCheckedAt: string;
  measuresFreshUntil: string;
  tradeRemedyWatchState: TradeRemedyWatchState;
  tradeRemedyCheckedAt: string;
  fxRateCheckedAt: string;
}

export interface FreshnessWindowConfig {
  structureDays: number;
  measuresDays: number;
  remedyDays: number;
  fxDays: number;
}

export interface FreshnessSweepInput {
  uk10: string;
  vectors: FreshnessVectors;
  expectedImportWindow: { start: string; end: string };
  now: Date;
  configuredWindow: FreshnessWindowConfig;
  lastSuccessfulWatcherRuns?: Partial<Record<FreshnessVector, string>>;
}

export interface FreshnessVectorState {
  vector: FreshnessVector;
  freshUntil: string;
  passing: boolean;
  expiresBeforeImportWindow: boolean;
  watcherMissedWindow: boolean;
  stateReason: string | null;
}

export interface FreshnessCatchUpJob {
  vector: FreshnessVector;
  action: "reclassify" | "recalculate";
  affectedUk10Codes: string[];
  affectedCommodityFamilies: string[];
  reason: "watcher_missed_window" | "freshness_expired" | "trade_remedy_state_changed";
}

export interface FreshnessSweepResult {
  passing: boolean;
  expiredVectors: FreshnessVector[];
  blockers: string[];
  enqueueReclassification: boolean;
  enqueueRecalculation: boolean;
  blocksL6Readiness: boolean;
  classificationFreshUntil: string;
  tradeRemedyWatchState: TradeRemedyWatchState;
  vectorStates: Record<FreshnessVector, FreshnessVectorState>;
  catchUpJobs: FreshnessCatchUpJob[];
}

export interface TariffChangeEventInput {
  kind: "structure" | "measures";
  changedAt: string;
  affectedUk10Codes: string[];
  sourceReference: string;
}

export interface TariffChangeEvent {
  eventCode: typeof TARIFF_STRUCTURE_CHANGED;
  vector: "structure" | "measures";
  changedAt: string;
  affectedUk10Codes: string[];
  affectedCommodityFamilies: string[];
  sourceReference: string;
  enqueueReclassification: true;
  enqueueRecalculation: false;
}

export interface TradeRemedyCaseOpeningEventInput {
  openedAt: string;
  caseReference: string;
  affectedUk10Codes: string[];
  originCountries: string[];
}

export interface TradeRemedyCaseOpeningEvent {
  eventCode: typeof TRA_INVESTIGATION_OPEN;
  vector: "remedies";
  openedAt: string;
  caseReference: string;
  affectedUk10Codes: string[];
  affectedCommodityFamilies: string[];
  originCountries: string[];
  tradeRemedyWatchState: "investigation_open";
  blockers: [typeof TRADE_REMEDY_WATCH_STALE];
  enqueueReclassification: true;
  enqueueRecalculation: false;
}

export function sweepFreshness(input: FreshnessSweepInput): FreshnessSweepResult {
  const expectedImportEnd = parseIsoDate(
    input.expectedImportWindow.end,
    "expectedImportWindow.end",
  );
  const structureFreshUntil = addDays(
    parseIsoDate(input.vectors.tariffCheckedAt, "vectors.tariffCheckedAt"),
    input.configuredWindow.structureDays,
  );
  const measuresFreshUntil = parseIsoDate(
    input.vectors.measuresFreshUntil,
    "vectors.measuresFreshUntil",
  );
  const remediesFreshUntil = addDays(
    parseIsoDate(input.vectors.tradeRemedyCheckedAt, "vectors.tradeRemedyCheckedAt"),
    input.configuredWindow.remedyDays,
  );
  const fxFreshUntil = addDays(
    parseIsoDate(input.vectors.fxRateCheckedAt, "vectors.fxRateCheckedAt"),
    input.configuredWindow.fxDays,
  );

  const vectorStates: Record<FreshnessVector, FreshnessVectorState> = {
    structure: vectorState({
      vector: "structure",
      freshUntil: structureFreshUntil,
      expectedImportEnd,
      input,
    }),
    measures: vectorState({
      vector: "measures",
      freshUntil: measuresFreshUntil,
      expectedImportEnd,
      input,
    }),
    remedies: vectorState({
      vector: "remedies",
      freshUntil: remediesFreshUntil,
      expectedImportEnd,
      input,
      forcedFailure:
        input.vectors.tradeRemedyWatchState === "stable"
          ? null
          : "trade_remedy_state_changed",
    }),
    fx: vectorState({
      vector: "fx",
      freshUntil: fxFreshUntil,
      expectedImportEnd,
      input,
    }),
  };
  const expiredVectors = orderedVectors().filter(
    (vector) => !vectorStates[vector].passing,
  );
  const catchUpJobs = expiredVectors
    .filter((vector) => vectorStates[vector].watcherMissedWindow)
    .map((vector) => catchUpJob(vector, input.uk10, "watcher_missed_window"));
  const blockers = blockersFor(expiredVectors);
  const tradeRemedyWatchState = vectorStates.remedies.watcherMissedWindow
    ? "stale"
    : input.vectors.tradeRemedyWatchState;

  return {
    passing: expiredVectors.length === 0,
    expiredVectors,
    blockers,
    enqueueReclassification: expiredVectors.some((vector) => vector !== "fx"),
    enqueueRecalculation: expiredVectors.includes("fx"),
    blocksL6Readiness: expiredVectors.length > 0,
    classificationFreshUntil: earliestIso([
      structureFreshUntil,
      measuresFreshUntil,
      remediesFreshUntil,
    ]),
    tradeRemedyWatchState,
    vectorStates,
    catchUpJobs,
  };
}

export function createTariffChangeEvent(
  input: TariffChangeEventInput,
): TariffChangeEvent {
  return {
    eventCode: TARIFF_STRUCTURE_CHANGED,
    vector: input.kind,
    changedAt: canonicalIso(input.changedAt, "changedAt"),
    affectedUk10Codes: uniqueSorted(input.affectedUk10Codes),
    affectedCommodityFamilies: commodityFamilies(input.affectedUk10Codes),
    sourceReference: input.sourceReference,
    enqueueReclassification: true,
    enqueueRecalculation: false,
  };
}

export function createTradeRemedyCaseOpeningEvent(
  input: TradeRemedyCaseOpeningEventInput,
): TradeRemedyCaseOpeningEvent {
  return {
    eventCode: TRA_INVESTIGATION_OPEN,
    vector: "remedies",
    openedAt: canonicalIso(input.openedAt, "openedAt"),
    caseReference: input.caseReference,
    affectedUk10Codes: uniqueSorted(input.affectedUk10Codes),
    affectedCommodityFamilies: commodityFamilies(input.affectedUk10Codes),
    originCountries: uniqueSorted(input.originCountries),
    tradeRemedyWatchState: "investigation_open",
    blockers: [TRADE_REMEDY_WATCH_STALE],
    enqueueReclassification: true,
    enqueueRecalculation: false,
  };
}

function vectorState(opts: {
  vector: FreshnessVector;
  freshUntil: Date;
  expectedImportEnd: Date;
  input: FreshnessSweepInput;
  forcedFailure?: string | null;
}): FreshnessVectorState {
  const didMissWatcherWindow = watcherMissedWindow(opts.vector, opts.input);
  const expiresBeforeImportWindow = opts.freshUntil < opts.expectedImportEnd;
  const stateReason =
    opts.forcedFailure ??
    (didMissWatcherWindow
      ? "watcher_missed_window"
      : expiresBeforeImportWindow
        ? "freshness_expired"
        : null);

  return {
    vector: opts.vector,
    freshUntil: opts.freshUntil.toISOString(),
    passing: stateReason === null,
    expiresBeforeImportWindow,
    watcherMissedWindow: didMissWatcherWindow,
    stateReason,
  };
}

function watcherMissedWindow(
  vector: FreshnessVector,
  input: FreshnessSweepInput,
): boolean {
  const lastSuccessfulRunAt = input.lastSuccessfulWatcherRuns?.[vector];
  if (lastSuccessfulRunAt === undefined) {
    return false;
  }

  const freshnessDays = freshnessWindowDays(vector, input.configuredWindow);
  const dueAt = addDays(
    parseIsoDate(lastSuccessfulRunAt, `lastSuccessfulWatcherRuns.${vector}`),
    freshnessDays,
  );
  return dueAt < input.now;
}

function freshnessWindowDays(
  vector: FreshnessVector,
  configuredWindow: FreshnessWindowConfig,
): number {
  switch (vector) {
    case "structure":
      return configuredWindow.structureDays;
    case "measures":
      return configuredWindow.measuresDays;
    case "remedies":
      return configuredWindow.remedyDays;
    case "fx":
      return configuredWindow.fxDays;
  }
}

function catchUpJob(
  vector: FreshnessVector,
  uk10: string,
  reason: FreshnessCatchUpJob["reason"],
): FreshnessCatchUpJob {
  return {
    vector,
    action: vector === "fx" ? "recalculate" : "reclassify",
    affectedUk10Codes: [uk10],
    affectedCommodityFamilies: commodityFamilies([uk10]),
    reason,
  };
}

function blockersFor(expiredVectors: FreshnessVector[]): string[] {
  const blockers = expiredVectors.map(
    (vector) => `CLASSIFICATION_FRESHNESS_EXPIRED:${vector}`,
  );
  if (expiredVectors.includes("remedies")) {
    blockers.push(TRADE_REMEDY_WATCH_STALE);
  }
  return uniqueSorted(blockers);
}

function orderedVectors(): FreshnessVector[] {
  return ["structure", "measures", "remedies", "fx"];
}

function earliestIso(dates: Date[]): string {
  return new Date(Math.min(...dates.map((date) => date.getTime()))).toISOString();
}

function addDays(date: Date, days: number): Date {
  return new Date(date.getTime() + days * MS_PER_DAY);
}

function parseIsoDate(value: string, field: string): Date {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    throw new Error(`${field} must be an ISO 8601 timestamp`);
  }
  return parsed;
}

function canonicalIso(value: string, field: string): string {
  return parseIsoDate(value, field).toISOString();
}

function commodityFamilies(uk10Codes: string[]): string[] {
  return uniqueSorted(uk10Codes.map((code) => code.slice(0, 4)));
}

function uniqueSorted(values: string[]): string[] {
  return [...new Set(values)].sort((left, right) => left.localeCompare(right));
}
