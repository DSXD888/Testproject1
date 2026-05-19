import { readFile } from "node:fs/promises";
import { isAbsolute } from "node:path";
import { pathToFileURL } from "node:url";
import type { FreightEstimate, ModeSelection } from "./freight-types";

export const SEARATES_SKILL_ID = "landed-cost/searates-freight-estimate" as const;

export type SearatesFailureCode =
  | "SEARATES_NO_QUOTE"
  | "SEARATES_UNSUPPORTED_MODE"
  | "SEARATES_SCHEMA_DRIFT"
  | "SEARATES_PERSISTENT_CDP_REQUIRED"
  | "SEARATES_RATE_LIMITED";

export interface SearatesEstimateOptions {
  mode: ModeSelection["mode"];
  originPort: string;
  destinationPort: string;
  weightKg: number;
  volumeM3: number;
  cassettePath?: string;
  checkedAt?: string;
  evidenceTranscriptPath?: string;
}

export interface SearatesAdapter {
  estimate(opts: {
    mode: ModeSelection["mode"];
    originPort: string;
    destinationPort: string;
    weightKg: number;
    volumeM3: number;
    cassettePath?: string;
  }): Promise<FreightEstimate>;
}

export interface SearatesSkillInvocation {
  skillId: typeof SEARATES_SKILL_ID;
  mode: "container" | "loose_cargo";
  originPort: string;
  destinationPort: string;
  weightKg: number;
  volumeM3: number;
}

export interface SearatesSkillRunner {
  invokeSearates(
    opts: SearatesSkillInvocation,
  ): Promise<SearatesSkillEnvelope | SearatesSkillResult>;
}

interface SearatesSkillEnvelope {
  result?: unknown;
  output?: unknown;
  run_manifest_path?: unknown;
  runManifestPath?: unknown;
  recorded_at?: unknown;
  checkedAt?: unknown;
}

interface SearatesSkillResult {
  [key: string]: unknown;
  freight_status?: unknown;
  cdp_state?: unknown;
  freight_currency?: unknown;
  freight_amount_minor?: unknown;
  freight_low_amount_minor?: unknown;
  freight_base_amount_minor?: unknown;
  freight_high_amount_minor?: unknown;
  quote_expired?: unknown;
  exchange_rates_seen?: unknown;
  warnings?: unknown;
  blockers?: unknown;
}

interface SearatesEvidence {
  result: SearatesSkillResult;
  recordedAt?: string;
  evidenceTranscriptPath?: string;
}

const SEARATES_CASSETTE_ROOT = new URL(
  "../../../../tests/fixtures/cassettes/l5/freight/",
  import.meta.url,
);

const SUPPORTED_SEARATES_MODES = new Set<ModeSelection["mode"]>([
  "sea_lcl",
  "sea_fcl",
]);

export function createSearatesAdapter(runner?: SearatesSkillRunner): SearatesAdapter {
  return {
    async estimate(opts) {
      const checkedAt = new Date().toISOString();

      if (!SUPPORTED_SEARATES_MODES.has(opts.mode)) {
        return searatesFailure(opts.mode, "SEARATES_UNSUPPORTED_MODE", checkedAt);
      }

      if (!Number.isFinite(opts.weightKg) || opts.weightKg <= 0) {
        return searatesFailure(opts.mode, "SEARATES_SCHEMA_DRIFT", checkedAt);
      }

      if (!Number.isFinite(opts.volumeM3) || opts.volumeM3 <= 0) {
        return searatesFailure(opts.mode, "SEARATES_SCHEMA_DRIFT", checkedAt);
      }

      if (opts.cassettePath !== undefined) {
        const evidence = await readSearatesEvidence(opts.cassettePath);
        return normalizeSearatesSkillResult(evidence, {
          mode: opts.mode,
          checkedAt,
          evidenceTranscriptPath: evidence.evidenceTranscriptPath,
        });
      }

      if (runner === undefined) {
        return searatesFailure(
          opts.mode,
          "SEARATES_PERSISTENT_CDP_REQUIRED",
          checkedAt,
        );
      }

      const invocation: SearatesSkillInvocation = {
        skillId: SEARATES_SKILL_ID,
        mode: opts.mode === "sea_fcl" ? "container" : "loose_cargo",
        originPort: opts.originPort,
        destinationPort: opts.destinationPort,
        weightKg: opts.weightKg,
        volumeM3: opts.volumeM3,
      };

      const envelope = await runner.invokeSearates(invocation);
      const result = unwrapSearatesResult(envelope);
      return normalizeSearatesSkillResult(result, {
        mode: opts.mode,
        checkedAt: readString(envelope, "recorded_at") ?? checkedAt,
        evidenceTranscriptPath:
          readString(envelope, "run_manifest_path") ??
          readString(envelope, "runManifestPath"),
      });
    },
  };
}

export function searatesUsesPaidApiCredentials(): false {
  return false;
}

export function normalizeSearatesSkillResult(
  resultOrEvidence: SearatesSkillResult | SearatesEvidence,
  opts: {
    mode: ModeSelection["mode"];
    checkedAt: string;
    evidenceTranscriptPath?: string;
  },
): FreightEstimate {
  const evidence = isSearatesEvidence(resultOrEvidence)
    ? resultOrEvidence
    : { result: resultOrEvidence };
  const result = evidence.result;
  const checkedAt = evidence.recordedAt ?? opts.checkedAt;
  const transcriptPath = evidence.evidenceTranscriptPath ?? opts.evidenceTranscriptPath;

  const blockers = readIssueCodes(result.blockers);
  const warnings = readIssueCodes(result.warnings);
  const issueCodes = [...blockers, ...warnings];
  const cdpState = readString(result, "cdp_state");
  const status = readString(result, "freight_status");

  if (cdpState !== undefined && cdpState !== "connected") {
    return searatesFailure(
      opts.mode,
      "SEARATES_PERSISTENT_CDP_REQUIRED",
      checkedAt,
      transcriptPath,
    );
  }

  if (status !== "quoted" || result.quote_expired === true) {
    return searatesFailure(
      opts.mode,
      mapSearatesFailure(issueCodes),
      checkedAt,
      transcriptPath,
    );
  }

  const currency = readString(result, "freight_currency");
  const baseAmountMinor = readAmountMinor(
    result.freight_base_amount_minor ?? result.freight_amount_minor,
  );
  const lowAmountMinor = readAmountMinor(result.freight_low_amount_minor);
  const highAmountMinor = readAmountMinor(result.freight_high_amount_minor);

  if (
    currency === undefined ||
    baseAmountMinor === null ||
    (result.freight_low_amount_minor !== null && lowAmountMinor === null) ||
    (result.freight_high_amount_minor !== null && highAmountMinor === null)
  ) {
    return searatesFailure(opts.mode, "SEARATES_SCHEMA_DRIFT", checkedAt, transcriptPath);
  }

  const fx = readFxToGbp(currency, result.exchange_rates_seen);
  if (fx === null) {
    return searatesFailure(opts.mode, "SEARATES_SCHEMA_DRIFT", checkedAt, transcriptPath);
  }

  return {
    provider: "searates",
    mode: opts.mode,
    lowGbpPence: lowAmountMinor === null ? null : toGbpPence(lowAmountMinor, fx),
    baseGbpPence: toGbpPence(baseAmountMinor, fx),
    highGbpPence: highAmountMinor === null ? null : toGbpPence(highAmountMinor, fx),
    evidenceTranscriptPath: transcriptPath,
    checkedAt,
  };
}

function searatesFailure(
  mode: ModeSelection["mode"],
  failureCode: SearatesFailureCode,
  checkedAt: string,
  evidenceTranscriptPath?: string,
): FreightEstimate {
  return {
    provider: "searates",
    mode,
    lowGbpPence: null,
    baseGbpPence: null,
    highGbpPence: null,
    failureCode,
    evidenceTranscriptPath,
    checkedAt,
  };
}

function mapSearatesFailure(issueCodes: string[]): SearatesFailureCode {
  const normalized = issueCodes.map((code) => code.toUpperCase());

  if (normalized.some((code) => code.includes("UNSUPPORTED_MODE"))) {
    return "SEARATES_UNSUPPORTED_MODE";
  }

  if (
    normalized.some(
      (code) =>
        code.includes("RATE_LIMIT") ||
        code.includes("TOO_MANY_REQUESTS") ||
        code.includes("HTTP_429") ||
        code === "429",
    )
  ) {
    return "SEARATES_RATE_LIMITED";
  }

  if (
    normalized.some(
      (code) =>
        code.includes("SCHEMA") ||
        code.includes("PAGE_SHAPE") ||
        code.includes("SELECTOR") ||
        code.includes("DOM_DRIFT"),
    )
  ) {
    return "SEARATES_SCHEMA_DRIFT";
  }

  if (
    normalized.some(
      (code) =>
        code.includes("CDP") ||
        code.includes("SESSION_EXPIRED") ||
        code.includes("SESSION_REFRESH") ||
        code.includes("CAPTCHA"),
    )
  ) {
    return "SEARATES_PERSISTENT_CDP_REQUIRED";
  }

  return "SEARATES_NO_QUOTE";
}

async function readSearatesEvidence(cassettePath: string): Promise<SearatesEvidence> {
  const body = await readFileFromKnownRoots(cassettePath);
  let parsed: unknown;
  try {
    parsed = JSON.parse(body) as unknown;
  } catch (error) {
    throw new Error(`SeaRates cassette ${cassettePath} is not valid JSON`, {
      cause: error,
    });
  }

  if (!isObject(parsed)) {
    throw new Error(`SeaRates cassette ${cassettePath} is not a JSON object`);
  }

  const result = isObject(parsed.result)
    ? parsed.result
    : isObject(parsed.output)
      ? parsed.output
      : parsed;

  return {
    result,
    recordedAt: readString(parsed, "recorded_at") ?? readString(parsed, "checkedAt"),
    evidenceTranscriptPath:
      readString(parsed, "run_manifest_path") ??
      readString(parsed, "runManifestPath") ??
      readString(parsed, "evidenceTranscriptPath") ??
      readString(parsed, "body_path"),
  };
}

async function readFileFromKnownRoots(path: string): Promise<string> {
  const candidates = isAbsolute(path)
    ? [pathToFileURL(path)]
    : [new URL(path, SEARATES_CASSETTE_ROOT), new URL(path, import.meta.url)];

  let lastError: unknown;
  for (const candidate of candidates) {
    try {
      return await readFile(candidate, "utf8");
    } catch (error) {
      lastError = error;
    }
  }

  throw lastError;
}

function unwrapSearatesResult(
  envelope: SearatesSkillEnvelope | SearatesSkillResult,
): SearatesSkillResult {
  if (isObject(envelope.result)) {
    return envelope.result;
  }

  if (isObject(envelope.output)) {
    return envelope.output;
  }

  if (isObject(envelope)) {
    return envelope;
  }

  return {};
}

function isSearatesEvidence(
  value: SearatesSkillResult | SearatesEvidence,
): value is SearatesEvidence {
  return isObject(value) && isObject(value.result);
}

function readIssueCodes(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .map((item) => {
      if (typeof item === "string") {
        return item;
      }

      if (isObject(item)) {
        return readString(item, "code");
      }

      return undefined;
    })
    .filter((code): code is string => code !== undefined && code.length > 0);
}

function readString(value: unknown, key: string): string | undefined {
  if (!isObject(value)) {
    return undefined;
  }

  const raw = value[key];
  return typeof raw === "string" && raw.length > 0 ? raw : undefined;
}

function readAmountMinor(value: unknown): number | null {
  return typeof value === "number" && Number.isInteger(value) && value >= 0
    ? value
    : null;
}

function readFxToGbp(currency: string, exchangeRates: unknown): number | null {
  const normalizedCurrency = currency.toUpperCase();

  if (normalizedCurrency === "GBP") {
    return 1;
  }

  if (!Array.isArray(exchangeRates)) {
    return null;
  }

  for (const rate of exchangeRates) {
    if (!isObject(rate)) {
      continue;
    }

    const from = readNestedCurrency(rate, "from");
    const to = readNestedCurrency(rate, "to");
    const exchangeRate = rate.exchangeRate;

    if (
      from === normalizedCurrency &&
      to === "GBP" &&
      typeof exchangeRate === "number" &&
      Number.isFinite(exchangeRate) &&
      exchangeRate > 0
    ) {
      return exchangeRate;
    }
  }

  return null;
}

function readNestedCurrency(value: Record<string, unknown>, key: string): string | null {
  const raw = value[key];
  if (typeof raw === "string") {
    return raw.toUpperCase();
  }

  if (isObject(raw)) {
    return readString(raw, "value")?.toUpperCase() ?? null;
  }

  return null;
}

function toGbpPence(minorAmount: number, fx: number): number {
  return Math.round(minorAmount * fx);
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
