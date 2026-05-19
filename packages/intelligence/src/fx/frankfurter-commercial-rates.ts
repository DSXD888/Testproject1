import { readFile } from "node:fs/promises";

import type { FxBaseCurrency, FxSnapshot } from "./fx-types";

export type FrankfurterCommercialBaseCurrency = Exclude<FxBaseCurrency, "GBP">;

export type FrankfurterFxMissingReason =
  | "missing_gbp_rate"
  | "parse_failure"
  | "upstream_unavailable"
  | "last_valid_expired";

export interface FrankfurterFetcher {
  fetchSpot(opts: {
    baseCurrency: FrankfurterCommercialBaseCurrency;
    cassettePath?: string;
  }): Promise<FxSnapshot>;

  fetchSpotResult(opts: FrankfurterFetchOptions): Promise<FrankfurterFxResult>;

  getLastValid(opts: {
    baseCurrency: string;
    maxAgeSeconds: number;
    now?: Date;
  }): FxSnapshot | null;
}

export interface FrankfurterFetchOptions {
  baseCurrency: FrankfurterCommercialBaseCurrency;
  cassettePath?: string;
  fallbackCassettePath?: string;
  maxAgeSeconds?: number;
  baseUrl?: string;
  env?: Record<string, string | undefined>;
  checkedAt?: string;
  now?: Date;
  snapshotVersion?: number;
}

export interface FrankfurterProbeOptions extends FrankfurterFetchOptions {
  env: Record<string, string | undefined>;
}

export interface FrankfurterFxFailure {
  ok: false;
  errorCode: "FX_RATE_MISSING";
  reason: FrankfurterFxMissingReason;
  message: string;
  sourceUrl: string;
  warning?: string;
  bidReadinessImpact: "warning" | "blocker";
  commercialEstimateConfidenceBpsReduction: number;
}

export interface FrankfurterFxSuccess {
  ok: true;
  snapshot: FxSnapshot;
  staleness: "fresh" | "last_valid";
  warning?: string;
  commercialEstimateConfidenceBpsReduction: number;
}

export type FrankfurterFxResult = FrankfurterFxSuccess | FrankfurterFxFailure;

export interface FrankfurterCassetteEnvelope {
  request: {
    url: string;
  };
  response: {
    status: number;
    body_path: string;
  };
  recorded_at: string;
}

export interface FrankfurterPublicProbeResult {
  probeUrl: string;
  result: FrankfurterFxResult;
}

interface FrankfurterParsedBody {
  amount?: unknown;
  base?: unknown;
  date?: unknown;
  rates?: unknown;
}

const DEFAULT_FRANKFURTER_BASE_URL = "https://api.frankfurter.app";
const LAST_VALID_CONFIDENCE_REDUCTION_BPS = 1000;
const MISSING_SPOT_CONFIDENCE_REDUCTION_BPS = 2500;

const CASSETTE_ROOT = new URL(
  "../../../../tests/fixtures/cassettes/l5/fx/",
  import.meta.url,
);

export function resolveFrankfurterBaseUrl(
  env: Record<string, string | undefined> = {},
): string {
  return (env.FRANKFURTER_BASE_URL ?? DEFAULT_FRANKFURTER_BASE_URL).replace(
    /\/+$/,
    "",
  );
}

export function buildFrankfurterSpotUrl(opts: {
  baseCurrency: FrankfurterCommercialBaseCurrency;
  baseUrl?: string;
  env?: Record<string, string | undefined>;
}): string {
  const baseUrl = opts.baseUrl ?? resolveFrankfurterBaseUrl(opts.env);
  return `${baseUrl}/latest?from=${opts.baseCurrency}&to=GBP`;
}

export function createFrankfurterFetcher(): FrankfurterFetcher {
  const lastValid = new Map<string, FxSnapshot>();

  return {
    async fetchSpot(opts) {
      const result = await fetchFrankfurterCommercialSpot({
        ...opts,
        lastValid,
      });
      if (result.ok) {
        return result.snapshot;
      }

      throw new Error(`${result.errorCode}: ${result.message}`);
    },

    fetchSpotResult(opts) {
      return fetchFrankfurterCommercialSpot({
        ...opts,
        lastValid,
      });
    },

    getLastValid(opts) {
      return findLastValid(lastValid, opts);
    },
  };
}

export async function probeFrankfurterPublicEndpoint(
  opts: FrankfurterProbeOptions,
): Promise<FrankfurterPublicProbeResult> {
  const probeUrl = buildFrankfurterSpotUrl({
    baseCurrency: opts.baseCurrency,
    baseUrl: opts.baseUrl,
    env: opts.env,
  });
  const result = await fetchFrankfurterCommercialSpot({
    ...opts,
    baseUrl: resolveFrankfurterBaseUrl(opts.env),
  });

  return { probeUrl, result };
}

export async function fetchFrankfurterCommercialSpot(
  opts: FrankfurterFetchOptions & {
    lastValid?: Map<string, FxSnapshot>;
  },
): Promise<FrankfurterFxResult> {
  const sourceUrl = buildFrankfurterSpotUrl(opts);
  if (opts.cassettePath) {
    const cassette = await readCassette(opts.cassettePath);
    if (cassette.response.status !== 200) {
      return recoverFromUnavailable({
        ...opts,
        sourceUrl: cassette.request.url,
        status: cassette.response.status,
      });
    }

    const body = await readFile(
      new URL(cassette.response.body_path, CASSETTE_ROOT),
      "utf8",
    );
    return parseAndRecordFrankfurterSpot({
      body,
      baseCurrency: opts.baseCurrency,
      sourceUrl: cassette.request.url,
      checkedAt: opts.checkedAt ?? cassette.recorded_at,
      snapshotVersion: opts.snapshotVersion,
      lastValid: opts.lastValid,
    });
  }

  try {
    const response = await fetch(sourceUrl, {
      redirect: "follow",
      headers: {
        accept: "application/json",
        "user-agent": "testproject1-l5-frankfurter-fx/1",
      },
    });
    if (!response.ok) {
      return recoverFromUnavailable({
        ...opts,
        sourceUrl,
        status: response.status,
      });
    }

    return parseAndRecordFrankfurterSpot({
      body: await response.text(),
      baseCurrency: opts.baseCurrency,
      sourceUrl,
      checkedAt: opts.checkedAt ?? new Date().toISOString(),
      snapshotVersion: opts.snapshotVersion,
      lastValid: opts.lastValid,
    });
  } catch (error) {
    return recoverFromUnavailable({
      ...opts,
      sourceUrl,
      status: null,
      cause: errorMessage(error),
    });
  }
}

export function parseFrankfurterPublication(opts: {
  body: string;
  baseCurrency: FrankfurterCommercialBaseCurrency;
  sourceUrl: string;
  checkedAt: string;
  snapshotVersion?: number;
}): FrankfurterFxResult {
  return parseAndRecordFrankfurterSpot(opts);
}

async function recoverFromUnavailable(
  opts: FrankfurterFetchOptions & {
    sourceUrl: string;
    status: number | null;
    cause?: string;
    lastValid?: Map<string, FxSnapshot>;
  },
): Promise<FrankfurterFxResult> {
  const warning = unavailableWarning(opts.status, opts.cause);
  if (opts.fallbackCassettePath) {
    const fallback = await fetchFrankfurterCommercialSpot({
      ...opts,
      cassettePath: opts.fallbackCassettePath,
      fallbackCassettePath: undefined,
    });
    if (fallback.ok) {
      return {
        ...fallback,
        warning: `${warning}; used cassette fallback`,
      };
    }
  }

  const lastValid =
    opts.lastValid && opts.maxAgeSeconds
      ? findLastValid(opts.lastValid, {
          baseCurrency: opts.baseCurrency,
          maxAgeSeconds: opts.maxAgeSeconds,
          now: opts.now,
        })
      : null;
  if (lastValid) {
    return {
      ok: true,
      snapshot: lastValid,
      staleness: "last_valid",
      warning: `${warning}; using TTL-valid last commercial FX snapshot`,
      commercialEstimateConfidenceBpsReduction:
        LAST_VALID_CONFIDENCE_REDUCTION_BPS,
    };
  }

  return frankfurterMissing({
    reason: opts.lastValid ? "last_valid_expired" : "upstream_unavailable",
    message: `${warning}; no TTL-valid Frankfurter commercial FX snapshot is available`,
    sourceUrl: opts.sourceUrl,
    warning,
    bidReadinessImpact: "blocker",
    commercialEstimateConfidenceBpsReduction:
      MISSING_SPOT_CONFIDENCE_REDUCTION_BPS,
  });
}

function parseAndRecordFrankfurterSpot(opts: {
  body: string;
  baseCurrency: FrankfurterCommercialBaseCurrency;
  sourceUrl: string;
  checkedAt: string;
  snapshotVersion?: number;
  lastValid?: Map<string, FxSnapshot>;
}): FrankfurterFxResult {
  const parsed = parseFrankfurterBody(opts.body);
  if (parsed.ok === false) {
    return frankfurterMissing({
      reason: parsed.reason,
      message: parsed.message,
      sourceUrl: opts.sourceUrl,
      bidReadinessImpact: "blocker",
      commercialEstimateConfidenceBpsReduction:
        MISSING_SPOT_CONFIDENCE_REDUCTION_BPS,
    });
  }

  const rate = parsed.rate;
  const effectiveDate = parsed.effectiveDate;
  const version = opts.snapshotVersion ?? 1;
  const snapshot: FxSnapshot = {
    basis: "frankfurter_commercial_spot",
    effectiveDate,
    baseCurrency: opts.baseCurrency,
    quoteCurrency: "GBP",
    rate,
    rateBps: Math.round(rate * 10000),
    sourceUrl: opts.sourceUrl,
    checkedAt: opts.checkedAt,
    snapshotId: `frankfurter-${effectiveDate}-${opts.baseCurrency}-GBP-v${version}`,
  };
  opts.lastValid?.set(lastValidKey(opts.baseCurrency), snapshot);

  return {
    ok: true,
    snapshot,
    staleness: "fresh",
    commercialEstimateConfidenceBpsReduction: 0,
  };
}

function parseFrankfurterBody(
  body: string,
):
  | {
      ok: true;
      rate: number;
      effectiveDate: string;
    }
  | {
      ok: false;
      reason: "missing_gbp_rate" | "parse_failure";
      message: string;
    } {
  let parsed: FrankfurterParsedBody;
  try {
    parsed = JSON.parse(body) as FrankfurterParsedBody;
  } catch (error) {
    return {
      ok: false,
      reason: "parse_failure",
      message: `Frankfurter response is malformed JSON: ${errorMessage(error)}`,
    };
  }

  if (parsed.base !== "USD" && parsed.base !== "EUR" && parsed.base !== "CNY") {
    return {
      ok: false,
      reason: "parse_failure",
      message: "Frankfurter response has an unsupported base currency",
    };
  }
  if (typeof parsed.date !== "string") {
    return {
      ok: false,
      reason: "parse_failure",
      message: "Frankfurter response is missing a date",
    };
  }
  if (!isRecord(parsed.rates) || !("GBP" in parsed.rates)) {
    return {
      ok: false,
      reason: "missing_gbp_rate",
      message: "Frankfurter response is missing rates.GBP",
    };
  }

  const rate = parsed.rates.GBP;
  if (typeof rate !== "number" || !Number.isFinite(rate) || rate <= 0) {
    return {
      ok: false,
      reason: "parse_failure",
      message: "Frankfurter response has an invalid rates.GBP value",
    };
  }

  return {
    ok: true,
    rate,
    effectiveDate: parsed.date,
  };
}

async function readCassette(
  cassettePath: string,
): Promise<FrankfurterCassetteEnvelope> {
  try {
    return JSON.parse(
      await readFile(new URL(cassettePath, CASSETTE_ROOT), "utf8"),
    ) as FrankfurterCassetteEnvelope;
  } catch (error) {
    throw new Error(
      `Frankfurter FX cassette ${cassettePath} is not valid JSON: ${errorMessage(error)}`,
    );
  }
}

function findLastValid(
  lastValid: Map<string, FxSnapshot>,
  opts: {
    baseCurrency: string;
    maxAgeSeconds: number;
    now?: Date;
  },
): FxSnapshot | null {
  const snapshot = lastValid.get(lastValidKey(opts.baseCurrency));
  if (!snapshot) {
    return null;
  }

  const checkedAtMs = Date.parse(snapshot.checkedAt);
  if (!Number.isFinite(checkedAtMs)) {
    return null;
  }

  const nowMs = (opts.now ?? new Date()).getTime();
  if (nowMs - checkedAtMs > opts.maxAgeSeconds * 1000) {
    return null;
  }

  return snapshot;
}

function lastValidKey(baseCurrency: string): string {
  return `${baseCurrency}-GBP`;
}

function unavailableWarning(status: number | null, cause?: string): string {
  if (status) {
    return `[WARN] Frankfurter commercial FX returned HTTP ${status}`;
  }

  return `[WARN] Frankfurter commercial FX request failed: ${cause ?? "unknown error"}`;
}

function frankfurterMissing(opts: {
  reason: FrankfurterFxMissingReason;
  message: string;
  sourceUrl: string;
  warning?: string;
  bidReadinessImpact: "warning" | "blocker";
  commercialEstimateConfidenceBpsReduction: number;
}): FrankfurterFxFailure {
  return {
    ok: false,
    errorCode: "FX_RATE_MISSING",
    reason: opts.reason,
    message: opts.message,
    sourceUrl: opts.sourceUrl,
    warning: opts.warning,
    bidReadinessImpact: opts.bidReadinessImpact,
    commercialEstimateConfidenceBpsReduction:
      opts.commercialEstimateConfidenceBpsReduction,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
