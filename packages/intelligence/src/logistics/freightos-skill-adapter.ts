import { readFile } from "node:fs/promises";
import { isAbsolute } from "node:path";
import { pathToFileURL } from "node:url";

import type { FreightEstimate, ModeSelection } from "./freight-types";

export const FREIGHTOS_SKILL_ID = "landed-cost/freightos-freight-estimate";

export type FreightosFailureCode =
  | "FREIGHTOS_NO_QUOTE"
  | "FREIGHTOS_UNSUPPORTED_MODE"
  | "FREIGHTOS_PAGE_SHAPE_DRIFT"
  | "FREIGHTOS_SCHEMA_DRIFT"
  | "FREIGHTOS_AUTH_PROMPT";

export interface FreightosAdapter {
  estimate(opts: FreightosEstimateOptions): Promise<FreightEstimate>;
}

export interface FreightosEstimateOptions {
  mode: ModeSelection["mode"];
  originPort: string;
  destinationPort: string;
  weightKg: number;
  volumeM3: number;
  cassettePath?: string;
  checkedAt?: string;
  evidenceTranscriptPath?: string;
}

export interface FreightosSkillRunner {
  invokeFreightos(
    opts: FreightosSkillInvocation,
  ): Promise<FreightosSkillResult | FreightosSkillEnvelope>;
}

export interface FreightosSkillInvocation {
  skillId: typeof FREIGHTOS_SKILL_ID;
  mode: "container" | "loose_cargo";
  originPort: string;
  destinationPort: string;
  weightKg: number;
  volumeM3: number;
}

interface FreightosSkillEnvelope {
  result?: unknown;
  output?: unknown;
  run_manifest_path?: string;
  runManifestPath?: string;
  recorded_at?: string;
}

interface FreightosSkillResult {
  freight_status?: unknown;
  cdp_state?: unknown;
  freight_currency?: unknown;
  freight_base_amount_minor?: unknown;
  freight_low_amount_minor?: unknown;
  freight_high_amount_minor?: unknown;
  quote_expired?: unknown;
  exchange_rates_seen?: unknown;
  warnings?: unknown;
  blockers?: unknown;
}

interface FreightosEvidence {
  result: FreightosSkillResult;
  recordedAt?: string;
  evidenceTranscriptPath?: string;
}

const FREIGHTOS_EVIDENCE_ROOT = new URL(
  "../../../../docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate/",
  import.meta.url,
);

const FREIGHT_CASSETTE_ROOT = new URL(
  "../../../../tests/fixtures/cassettes/l5/freight/",
  import.meta.url,
);

const SUPPORTED_FREIGHTOS_MODES = new Set<ModeSelection["mode"]>([
  "sea_lcl",
  "sea_fcl",
]);

export function createFreightosAdapter(
  runner?: FreightosSkillRunner,
): FreightosAdapter {
  return {
    async estimate(opts) {
      validateRequest(opts);
      if (!SUPPORTED_FREIGHTOS_MODES.has(opts.mode)) {
        return freightosFailure("FREIGHTOS_UNSUPPORTED_MODE", opts);
      }

      if (opts.cassettePath) {
        return normalizeFreightosSkillResult(
          await readFreightosEvidence(opts.cassettePath),
          opts,
        );
      }

      if (!runner) {
        return freightosFailure("FREIGHTOS_SCHEMA_DRIFT", opts);
      }

      return normalizeFreightosSkillResult(
        unwrapFreightosEnvelope(
          await runner.invokeFreightos({
            skillId: FREIGHTOS_SKILL_ID,
            mode: opts.mode === "sea_fcl" ? "container" : "loose_cargo",
            originPort: opts.originPort,
            destinationPort: opts.destinationPort,
            weightKg: opts.weightKg,
            volumeM3: opts.volumeM3,
          }),
        ),
        opts,
      );
    },
  };
}

export function freightosUsesPaidApiCredentials(): false {
  return false;
}

export function normalizeFreightosSkillResult(
  evidence: FreightosEvidence | FreightosSkillResult,
  opts: FreightosEstimateOptions,
): FreightEstimate {
  const result = "result" in evidence ? evidence.result : evidence;
  const checkedAt =
    opts.checkedAt ??
    ("recordedAt" in evidence ? evidence.recordedAt : undefined) ??
    new Date().toISOString();
  const evidenceTranscriptPath =
    opts.evidenceTranscriptPath ??
    ("evidenceTranscriptPath" in evidence
      ? evidence.evidenceTranscriptPath
      : undefined);

  if (!isObject(result)) {
    return freightosFailure("FREIGHTOS_SCHEMA_DRIFT", opts, {
      checkedAt,
      evidenceTranscriptPath,
    });
  }

  const blockers = readIssueCodes(result.blockers);
  const warnings = readIssueCodes(result.warnings);
  const status = readString(result.freight_status);
  const cdpState = readString(result.cdp_state);
  if (status !== "quoted" || result.quote_expired === true) {
    return freightosFailure(mapFreightosFailure(blockers, warnings, cdpState), opts, {
      checkedAt,
      evidenceTranscriptPath,
    });
  }

  const currency = readString(result.freight_currency);
  const baseAmountMinor = result.freight_base_amount_minor;
  const baseGbpPence = amountToGbpPence(
    baseAmountMinor,
    currency,
    result.exchange_rates_seen,
  );
  if (baseGbpPence === null) {
    return freightosFailure(
      baseAmountMinor === null || baseAmountMinor === undefined
        ? "FREIGHTOS_NO_QUOTE"
        : "FREIGHTOS_SCHEMA_DRIFT",
      opts,
      {
        checkedAt,
        evidenceTranscriptPath,
      },
    );
  }

  const lowGbpPence = amountToGbpPence(
    result.freight_low_amount_minor,
    currency,
    result.exchange_rates_seen,
  );
  const highGbpPence = amountToGbpPence(
    result.freight_high_amount_minor,
    currency,
    result.exchange_rates_seen,
  );

  if (
    (result.freight_low_amount_minor !== null && lowGbpPence === null) ||
    (result.freight_high_amount_minor !== null && highGbpPence === null)
  ) {
    return freightosFailure("FREIGHTOS_SCHEMA_DRIFT", opts, {
      checkedAt,
      evidenceTranscriptPath,
    });
  }

  return {
    provider: "freightos",
    mode: opts.mode,
    lowGbpPence,
    baseGbpPence,
    highGbpPence,
    checkedAt,
    evidenceTranscriptPath,
  };
}

function validateRequest(opts: FreightosEstimateOptions): void {
  for (const [name, value] of [
    ["weightKg", opts.weightKg],
    ["volumeM3", opts.volumeM3],
  ] as const) {
    if (!Number.isFinite(value) || value < 0) {
      throw new Error(`Freightos ${name} must be a non-negative finite number`);
    }
  }
}

function freightosFailure(
  failureCode: FreightosFailureCode,
  opts: FreightosEstimateOptions,
  override: {
    checkedAt?: string;
    evidenceTranscriptPath?: string;
  } = {},
): FreightEstimate {
  return {
    provider: "freightos",
    mode: opts.mode,
    lowGbpPence: null,
    baseGbpPence: null,
    highGbpPence: null,
    failureCode,
    checkedAt: override.checkedAt ?? opts.checkedAt ?? new Date().toISOString(),
    evidenceTranscriptPath:
      override.evidenceTranscriptPath ??
      opts.evidenceTranscriptPath ??
      opts.cassettePath,
  };
}

async function readFreightosEvidence(
  cassettePath: string,
): Promise<FreightosEvidence> {
  const raw = await readFileFromKnownRoots(cassettePath);
  const parsed = parseJsonObject(raw, cassettePath);
  if (isObject(parsed.response) && typeof parsed.response.body_path === "string") {
    const body = await readFileFromKnownRoots(parsed.response.body_path);
    return {
      result: parseJsonObject(body, parsed.response.body_path),
      recordedAt: readString(parsed.recorded_at) ?? undefined,
      evidenceTranscriptPath: cassettePath,
    };
  }

  return {
    result: parsed,
    recordedAt: readString(parsed.recorded_at) ?? undefined,
    evidenceTranscriptPath: cassettePath,
  };
}

async function readFileFromKnownRoots(path: string): Promise<string> {
  if (isAbsolute(path)) {
    return readFile(pathToFileURL(path), "utf8");
  }

  const attempts = [
    new URL(path, FREIGHT_CASSETTE_ROOT),
    new URL(path, FREIGHTOS_EVIDENCE_ROOT),
    new URL(path, import.meta.url),
  ];
  let lastError: unknown;
  for (const attempt of attempts) {
    try {
      return await readFile(attempt, "utf8");
    } catch (error) {
      lastError = error;
    }
  }

  throw new Error(
    `Unable to read Freightos cassette ${path}: ${errorMessage(lastError)}`,
  );
}

function unwrapFreightosEnvelope(
  value: FreightosSkillResult | FreightosSkillEnvelope,
): FreightosEvidence {
  if (isObject(value) && ("result" in value || "output" in value)) {
    const result = value.result ?? value.output;
    return {
      result: isObject(result) ? result : {},
      recordedAt: readString(value.recorded_at) ?? undefined,
      evidenceTranscriptPath:
        readString(value.run_manifest_path) ?? readString(value.runManifestPath) ?? undefined,
    };
  }

  return { result: isObject(value) ? value : {} };
}

function mapFreightosFailure(
  blockers: string[],
  warnings: string[],
  cdpState: string | null,
): FreightosFailureCode {
  const codes = [...blockers, ...warnings].map((code) => code.toUpperCase());
  if (
    cdpState === "session_expired" ||
    codes.some((code) =>
      [
        "SESSION_REFRESH_FAILED",
        "FREIGHTOS_SESSION_EXPIRED",
        "CDP_CAPTCHA_INTERCEPTION",
        "FREIGHTOS_AUTH_PROMPT",
      ].includes(code),
    )
  ) {
    return "FREIGHTOS_AUTH_PROMPT";
  }

  if (
    codes.some(
      (code) =>
        code.includes("PAGE_SHAPE") ||
        code.includes("SELECTOR") ||
        code.includes("DOM_DRIFT") ||
        code === "CDP_CONNECT_FAILED",
    )
  ) {
    return "FREIGHTOS_PAGE_SHAPE_DRIFT";
  }

  if (codes.some((code) => code.includes("UNSUPPORTED_MODE"))) {
    return "FREIGHTOS_UNSUPPORTED_MODE";
  }

  if (codes.some((code) => code.includes("SCHEMA"))) {
    return "FREIGHTOS_SCHEMA_DRIFT";
  }

  return "FREIGHTOS_NO_QUOTE";
}

function amountToGbpPence(
  amountMinor: unknown,
  currency: string | null,
  exchangeRatesSeen: unknown,
): number | null {
  if (amountMinor === null || amountMinor === undefined) {
    return null;
  }

  if (!Number.isInteger(amountMinor) || amountMinor < 0 || !currency) {
    return null;
  }

  if (currency.toUpperCase() === "GBP") {
    return amountMinor;
  }

  const rate = findGbpConversionRate(currency, exchangeRatesSeen);
  if (rate === null) {
    return null;
  }

  return Math.round(amountMinor * rate);
}

function findGbpConversionRate(
  currency: string,
  exchangeRatesSeen: unknown,
): number | null {
  if (!Array.isArray(exchangeRatesSeen)) {
    return null;
  }

  const quote = currency.toUpperCase();
  for (const entry of exchangeRatesSeen) {
    if (!isObject(entry)) {
      continue;
    }
    const from = nestedCurrency(entry.from);
    const to = nestedCurrency(entry.to);
    const exchangeRate = entry.exchangeRate;
    if (!Number.isFinite(exchangeRate) || exchangeRate <= 0) {
      continue;
    }

    if (from === "GBP" && to === quote) {
      return 1 / exchangeRate;
    }
    if (from === quote && to === "GBP") {
      return exchangeRate;
    }
  }

  return null;
}

function nestedCurrency(value: unknown): string | null {
  if (!isObject(value) || typeof value.value !== "string") {
    return null;
  }

  return value.value.toUpperCase();
}

function readIssueCodes(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value.flatMap((issue) => {
    if (!isObject(issue) || typeof issue.code !== "string") {
      return [];
    }

    return [issue.code];
  });
}

function parseJsonObject(raw: string, path: string): FreightosSkillResult {
  try {
    const parsed: unknown = JSON.parse(raw);
    if (isObject(parsed)) {
      return parsed;
    }
  } catch (error) {
    throw new Error(
      `Freightos cassette ${path} is not valid JSON: ${errorMessage(error)}`,
    );
  }

  throw new Error(`Freightos cassette ${path} is not a JSON object`);
}

function readString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
