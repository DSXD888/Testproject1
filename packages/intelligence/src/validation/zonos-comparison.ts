import { readFileSync } from "node:fs";

export interface LandedCostEstimate {
  dutyPence: number;
  vatPence: number;
  freightPence: number;
  totalPence: number;
}

export interface ComparisonReport {
  adapter: "zonos";
  l5Output: LandedCostEstimate;
  externalOutput: LandedCostEstimate;
  divergenceBps: {
    duty: number;
    vat: number;
    freight: number;
    total: number;
  };
  notes: string[];
}

interface ZonosFixture {
  adapter: "zonos";
  captured_at: string;
  input_scenario: string;
  source_kind: "captured_fixture";
  component_pence: LandedCostEstimate;
  notes?: string[];
}

export function compare(opts: {
  l5Output: LandedCostEstimate;
  externalSampleFixturePath: string;
}): ComparisonReport {
  const fixture = readFixture(opts.externalSampleFixturePath);
  const l5Output = normalizeEstimate(opts.l5Output, "l5Output");
  const externalOutput = normalizeEstimate(
    fixture.component_pence,
    "zonos fixture component_pence",
  );

  return {
    adapter: "zonos",
    l5Output,
    externalOutput,
    divergenceBps: divergenceBps(l5Output, externalOutput),
    notes: [
      `fixture-only comparison; captured_at=${fixture.captured_at}; scenario=${fixture.input_scenario}`,
      ...(fixture.notes ?? []),
    ],
  };
}

function readFixture(path: string): ZonosFixture {
  let parsed: Partial<ZonosFixture>;
  try {
    parsed = JSON.parse(readFileSync(path, "utf8")) as Partial<ZonosFixture>;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Invalid Zonos comparison fixture JSON: ${path}: ${message}`);
  }

  if (
    parsed.adapter !== "zonos" ||
    parsed.source_kind !== "captured_fixture" ||
    typeof parsed.captured_at !== "string" ||
    typeof parsed.input_scenario !== "string" ||
    parsed.component_pence === undefined
  ) {
    throw new Error(`Invalid Zonos comparison fixture: ${path}`);
  }

  return parsed as ZonosFixture;
}

function normalizeEstimate(
  estimate: LandedCostEstimate,
  context: string,
): LandedCostEstimate {
  const normalized = {
    dutyPence: estimate.dutyPence,
    vatPence: estimate.vatPence,
    freightPence: estimate.freightPence,
    totalPence: estimate.totalPence,
  };

  for (const [field, value] of Object.entries(normalized)) {
    if (!Number.isFinite(value) || value < 0) {
      throw new Error(`${context}.${field} must be a non-negative finite number`);
    }
  }

  return normalized;
}

function divergenceBps(
  l5Output: LandedCostEstimate,
  externalOutput: LandedCostEstimate,
): ComparisonReport["divergenceBps"] {
  return {
    duty: relativeBps(l5Output.dutyPence, externalOutput.dutyPence),
    vat: relativeBps(l5Output.vatPence, externalOutput.vatPence),
    freight: relativeBps(l5Output.freightPence, externalOutput.freightPence),
    total: relativeBps(l5Output.totalPence, externalOutput.totalPence),
  };
}

function relativeBps(l5Value: number, externalValue: number): number {
  return Math.round((Math.abs(l5Value - externalValue) * 10_000) / Math.max(1, externalValue));
}
