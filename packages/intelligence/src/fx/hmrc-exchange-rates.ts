import { readFile } from "node:fs/promises";

import type { FxBaseCurrency, FxRecalculationTrigger, FxSnapshot } from "./fx-types";

export type HmrcCustomsBaseCurrency = Exclude<FxBaseCurrency, "GBP">;

export type HmrcFxMissingReason = "missing_month" | "parse_failure";

export interface HmrcMonthlyFetcher {
  fetchSnapshot(opts: {
    baseCurrency: HmrcCustomsBaseCurrency;
    effectiveMonth: string;
    cassettePath?: string;
  }): Promise<FxSnapshot>;

  detectRollover(
    previous: FxSnapshot,
    now: Date,
  ): { rolledOver: boolean; newMonth?: string };

  detectCorrection(
    previous: FxSnapshot,
    fresh: FxSnapshot,
  ): { corrected: boolean };
}

export interface HmrcFxFailure {
  ok: false;
  errorCode: "FX_RATE_MISSING";
  reason: HmrcFxMissingReason;
  message: string;
  sourceUrl: string;
}

export interface HmrcFxSuccess {
  ok: true;
  snapshot: FxSnapshot;
}

export type HmrcFxResult = HmrcFxSuccess | HmrcFxFailure;

export interface HmrcRecalculationJob {
  trigger: FxRecalculationTrigger;
  previousSnapshotId: string;
  newSnapshotId: string;
  staleReason: "hmrc_monthly_rollover" | "hmrc_historical_correction";
}

export interface HmrcCassetteEnvelope {
  request: {
    url: string;
  };
  response: {
    body_path: string;
  };
  recorded_at: string;
}

export interface HmrcParseOptions {
  body: string;
  baseCurrency: HmrcCustomsBaseCurrency;
  effectiveMonth: string;
  sourceUrl: string;
  checkedAt: string;
  snapshotVersion?: number;
}

const DEFAULT_HMRC_EXCHANGE_RATES_URL =
  "https://www.trade-tariff.service.gov.uk/exchange_rates/monthly?year=2026";

const CASSETTE_ROOT = new URL(
  "../../../../tests/fixtures/cassettes/l5/fx/",
  import.meta.url,
);

export function resolveHmrcExchangeRatesUrl(
  env: Record<string, string | undefined> = {},
): string {
  return env.HMRC_EXCHANGE_RATES_URL ?? DEFAULT_HMRC_EXCHANGE_RATES_URL;
}

export function createHmrcMonthlyFetcher(): HmrcMonthlyFetcher {
  return {
    async fetchSnapshot(opts) {
      const result = await fetchHmrcMonthlySnapshot(opts);
      if (result.ok) {
        return result.snapshot;
      }

      throw new Error(`${result.errorCode}: ${result.message}`);
    },
    detectRollover: detectHmrcRollover,
    detectCorrection: detectHmrcCorrection,
  };
}

export async function fetchHmrcMonthlySnapshot(opts: {
  baseCurrency: HmrcCustomsBaseCurrency;
  effectiveMonth: string;
  cassettePath?: string;
  snapshotVersion?: number;
  exchangeRatesUrl?: string;
  checkedAt?: string;
}): Promise<HmrcFxResult> {
  if (opts.cassettePath) {
    const cassette = await readCassette(opts.cassettePath);
    const body = await readFile(
      new URL(cassette.response.body_path, CASSETTE_ROOT),
      "utf8",
    );
    return parseHmrcMonthlyPublication({
      body,
      baseCurrency: opts.baseCurrency,
      effectiveMonth: opts.effectiveMonth,
      sourceUrl: cassette.request.url,
      checkedAt: cassette.recorded_at,
      snapshotVersion: opts.snapshotVersion,
    });
  }

  const sourceUrl = opts.exchangeRatesUrl ?? DEFAULT_HMRC_EXCHANGE_RATES_URL;
  const response = await fetch(sourceUrl, {
    headers: {
      accept: "text/csv, application/json, application/xml, text/xml, text/html",
      "user-agent": "testproject1-l5-hmrc-fx/1",
    },
  });
  if (!response.ok) {
    return fxRateMissing(
      "missing_month",
      `HMRC monthly exchange-rate fetch returned HTTP ${response.status}`,
      sourceUrl,
    );
  }

  return parseHmrcMonthlyPublication({
    body: await response.text(),
    baseCurrency: opts.baseCurrency,
    effectiveMonth: opts.effectiveMonth,
    sourceUrl,
    checkedAt: opts.checkedAt ?? new Date().toISOString(),
    snapshotVersion: opts.snapshotVersion,
  });
}

export function parseHmrcMonthlyPublication(opts: HmrcParseOptions): HmrcFxResult {
  try {
    const rows = parseRows(opts.body);
    const match = rows.find(
      (row) =>
        row.month === opts.effectiveMonth &&
        row.currencyCode === opts.baseCurrency,
    );
    if (!match) {
      return fxRateMissing(
        "missing_month",
        `HMRC monthly publication has no ${opts.baseCurrency}/GBP rate for ${opts.effectiveMonth}`,
        opts.sourceUrl,
      );
    }

    const rate = Number(match.rate);
    if (!Number.isFinite(rate) || rate <= 0) {
      return fxRateMissing(
        "parse_failure",
        `HMRC monthly publication has an invalid ${opts.baseCurrency}/GBP rate for ${opts.effectiveMonth}`,
        opts.sourceUrl,
      );
    }

    const version = opts.snapshotVersion ?? 1;
    return {
      ok: true,
      snapshot: {
        basis: "hmrc_customs_monthly",
        effectiveMonth: opts.effectiveMonth,
        baseCurrency: opts.baseCurrency,
        quoteCurrency: "GBP",
        rate,
        rateBps: Math.round(rate * 10000),
        sourceUrl: opts.sourceUrl,
        checkedAt: opts.checkedAt,
        snapshotId: `hmrc-${opts.effectiveMonth}-${opts.baseCurrency}-GBP-v${version}`,
      },
    };
  } catch (error) {
    return fxRateMissing(
      "parse_failure",
      error instanceof Error ? error.message : String(error),
      opts.sourceUrl,
    );
  }
}

export function detectHmrcRollover(
  previous: FxSnapshot,
  now: Date,
): { rolledOver: boolean; newMonth?: string } {
  const previousMonth = previous.effectiveMonth;
  const currentMonth = now.toISOString().slice(0, 7);
  if (!previousMonth || currentMonth <= previousMonth) {
    return { rolledOver: false };
  }

  return {
    rolledOver: true,
    newMonth: currentMonth,
  };
}

export function planHmrcRolloverRecalculation(
  previous: FxSnapshot,
  fresh: FxSnapshot,
  now: Date,
): HmrcRecalculationJob[] {
  const rollover = detectHmrcRollover(previous, now);
  const newMonth = rollover.newMonth;
  if (
    !rollover.rolledOver ||
    !newMonth ||
    !previous.effectiveMonth ||
    fresh.effectiveMonth !== newMonth
  ) {
    return [];
  }

  return [
    {
      trigger: {
        kind: "monthly_rollover",
        previousMonth: previous.effectiveMonth,
        newMonth,
      },
      previousSnapshotId: previous.snapshotId,
      newSnapshotId: fresh.snapshotId,
      staleReason: "hmrc_monthly_rollover",
    },
  ];
}

export function detectHmrcCorrection(
  previous: FxSnapshot,
  fresh: FxSnapshot,
): { corrected: boolean } {
  return {
    corrected:
      previous.basis === "hmrc_customs_monthly" &&
      fresh.basis === "hmrc_customs_monthly" &&
      previous.effectiveMonth === fresh.effectiveMonth &&
      previous.baseCurrency === fresh.baseCurrency &&
      previous.quoteCurrency === fresh.quoteCurrency &&
      previous.rateBps !== fresh.rateBps,
  };
}

export function planHmrcCorrectionRecalculation(
  previous: FxSnapshot,
  fresh: FxSnapshot,
): HmrcRecalculationJob[] {
  if (!detectHmrcCorrection(previous, fresh).corrected || !fresh.effectiveMonth) {
    return [];
  }

  return [
    {
      trigger: {
        kind: "historical_correction",
        affectedMonth: fresh.effectiveMonth,
        previousSnapshotId: previous.snapshotId,
        newSnapshotId: fresh.snapshotId,
      },
      previousSnapshotId: previous.snapshotId,
      newSnapshotId: fresh.snapshotId,
      staleReason: "hmrc_historical_correction",
    },
  ];
}

interface ParsedHmrcRateRow {
  month: string;
  currencyCode: HmrcCustomsBaseCurrency;
  rate: string;
}

async function readCassette(cassettePath: string): Promise<HmrcCassetteEnvelope> {
  try {
    return JSON.parse(
      await readFile(new URL(cassettePath, CASSETTE_ROOT), "utf8"),
    ) as HmrcCassetteEnvelope;
  } catch (error) {
    throw new Error(
      `HMRC FX cassette ${cassettePath} is not valid JSON: ${errorMessage(error)}`,
    );
  }
}

function parseRows(body: string): ParsedHmrcRateRow[] {
  const trimmed = body.trim();
  if (!trimmed) {
    throw new Error("HMRC monthly publication body is empty");
  }

  if (trimmed.startsWith("{")) {
    return parseJsonRows(trimmed);
  }
  if (trimmed.startsWith("<")) {
    return parseXmlRows(trimmed);
  }

  return parseCsvRows(trimmed);
}

function parseJsonRows(body: string): ParsedHmrcRateRow[] {
  let parsed: {
    month?: unknown;
    rates?: Array<{ currency_code?: unknown; rate?: unknown }>;
  };
  try {
    parsed = JSON.parse(body) as {
      month?: unknown;
      rates?: Array<{ currency_code?: unknown; rate?: unknown }>;
    };
  } catch (error) {
    throw new Error(`HMRC JSON publication is malformed: ${errorMessage(error)}`);
  }
  if (typeof parsed.month !== "string" || !Array.isArray(parsed.rates)) {
    throw new Error("HMRC JSON publication missing month or rates array");
  }

  const publicationMonth = parsed.month;
  return parsed.rates.map((rate) => {
    if (typeof rate.currency_code !== "string" || typeof rate.rate !== "string") {
      throw new Error("HMRC JSON publication has malformed rate rows");
    }

    return {
      month: publicationMonth,
      currencyCode: toHmrcBaseCurrency(rate.currency_code),
      rate: rate.rate,
    };
  });
}

function parseCsvRows(body: string): ParsedHmrcRateRow[] {
  const lines = body.split(/\r?\n/).filter((line) => line.trim().length > 0);
  if (lines.length < 2) {
    throw new Error("HMRC CSV publication has no data rows");
  }

  const header = lines[0].split(",").map((column) => column.trim());
  const monthIndex = header.indexOf("month");
  const codeIndex = header.indexOf("currency_code");
  const rateIndex = header.indexOf("rate");
  if (monthIndex < 0 || codeIndex < 0 || rateIndex < 0) {
    throw new Error("HMRC CSV publication missing required columns");
  }

  return lines.slice(1).map((line) => {
    const columns = line.split(",").map((column) => column.trim());
    return {
      month: requiredColumn(columns, monthIndex, "month"),
      currencyCode: toHmrcBaseCurrency(
        requiredColumn(columns, codeIndex, "currency_code"),
      ),
      rate: requiredColumn(columns, rateIndex, "rate"),
    };
  });
}

function parseXmlRows(body: string): ParsedHmrcRateRow[] {
  const monthMatch = body.match(/month="([^"]+)"/);
  if (!monthMatch) {
    throw new Error("HMRC XML publication missing month attribute");
  }

  const rows: ParsedHmrcRateRow[] = [];
  const ratePattern =
    /<rate\s+currencyCode="([^"]+)"\s+value="([^"]+)"\s*\/>/g;
  for (const match of body.matchAll(ratePattern)) {
    rows.push({
      month: monthMatch[1],
      currencyCode: toHmrcBaseCurrency(match[1]),
      rate: match[2],
    });
  }

  if (rows.length === 0) {
    throw new Error("HMRC XML publication has no rate rows");
  }

  return rows;
}

function requiredColumn(columns: string[], index: number, name: string): string {
  const value = columns[index];
  if (!value) {
    throw new Error(`HMRC CSV publication missing ${name} value`);
  }

  return value;
}

function toHmrcBaseCurrency(currencyCode: string): HmrcCustomsBaseCurrency {
  if (currencyCode === "USD" || currencyCode === "EUR" || currencyCode === "CNY") {
    return currencyCode;
  }

  throw new Error(`Unsupported HMRC customs FX base currency ${currencyCode}`);
}

function fxRateMissing(
  reason: HmrcFxMissingReason,
  message: string,
  sourceUrl: string,
): HmrcFxFailure {
  return {
    ok: false,
    errorCode: "FX_RATE_MISSING",
    reason,
    message,
    sourceUrl,
  };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
