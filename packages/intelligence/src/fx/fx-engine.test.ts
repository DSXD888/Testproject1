import { describe, expect, it } from "bun:test";
import { createHash } from "node:crypto";

import type {
  FxBasisDelta,
  FxRecalculationTrigger,
  FxSnapshot,
} from "./fx-types";
import {
  createFrankfurterFetcher,
  fetchFrankfurterCommercialSpot,
  probeFrankfurterPublicEndpoint,
} from "./frankfurter-commercial-rates";
import {
  fetchHmrcMonthlySnapshot,
  planHmrcCorrectionRecalculation,
  planHmrcRolloverRecalculation,
} from "./hmrc-exchange-rates";

// [TEST_START] L5 FX engine scaffold.
// L5-T-fx_engine-001,005,006,007,009 land in l5-p2-hmrc-customs-fx-parser.
// L5-T-fx_engine-002,008,011 land in l5-p2-frankfurter-commercial-fx.
// L5-T-fx_engine-003,004,010,012 land in l5-p2-fx-basis-recalculation.

interface CassetteFixture {
  case_id: string;
  endpoint_family: string;
  request: {
    method: string;
    url: string;
    headers: Record<string, string>;
    body_sha256: string | null;
    authenticated: boolean;
  };
  request_fingerprint: string;
  response: {
    status: number;
    headers: Record<string, string>;
    body_path: string;
    body_sha256: string;
  };
  recorded_at: string;
  recorder: string;
  expected_failure_mode: string | null;
}

describe("fx-engine scaffold", () => {
  it("FxSnapshot, FxBasisDelta, and FxRecalculationTrigger compile structurally", () => {
    const customsSnapshot: FxSnapshot = {
      basis: "hmrc_customs_monthly",
      effectiveMonth: "2026-05",
      baseCurrency: "USD",
      quoteCurrency: "GBP",
      rate: 0.7834,
      rateBps: 7834,
      sourceUrl:
        "https://www.trade-tariff.service.gov.uk/exchange_rates/monthly?year=2026",
      checkedAt: "2026-05-19T00:00:00Z",
      snapshotId: "hmrc-2026-05-USD-GBP-v1",
    };
    const commercialSnapshot: FxSnapshot = {
      basis: "frankfurter_commercial_spot",
      effectiveDate: "2026-05-19",
      baseCurrency: "USD",
      quoteCurrency: "GBP",
      rate: 0.7871,
      rateBps: 7871,
      sourceUrl: "https://api.frankfurter.app/2026-05-19?from=USD&to=GBP",
      checkedAt: "2026-05-19T12:00:00Z",
      snapshotId: "frankfurter-2026-05-19-USD-GBP-v1",
    };
    const delta: FxBasisDelta = {
      customsSnapshot,
      commercialSnapshot,
      basisDeltaBps: 47,
      computedAt: "2026-05-19T12:01:00Z",
    };
    const trigger: FxRecalculationTrigger = {
      kind: "monthly_rollover",
      previousMonth: "2026-04",
      newMonth: "2026-05",
    };

    expect(delta.customsSnapshot.rateBps).toBe(7834);
    expect(delta.commercialSnapshot.rateBps).toBe(7871);
    expect(trigger.kind).toBe("monthly_rollover");
  });

  it("initial success cassettes follow the L5 cassette envelope", async () => {
    const cassettes = await Promise.all([
      readCassette("hmrc-monthly-current.json"),
      readCassette("frankfurter-usd-gbp.json"),
    ]);

    expect(cassettes.map((cassette) => cassette.endpoint_family)).toEqual([
      "hmrc_customs_monthly",
      "frankfurter_commercial_spot",
    ]);

    for (const cassette of cassettes) {
      expect(cassette.case_id.length).toBeGreaterThan(0);
      expect(cassette.request.method).toBe("GET");
      expect(cassette.request.authenticated).toBe(false);
      expect(cassette.request.headers.accept).toBeTruthy();
      expect(cassette.request_fingerprint.startsWith("sha256:")).toBe(true);
      expect(cassette.response.status).toBe(200);
      expect(cassette.response.body_path.length).toBeGreaterThan(0);
      expect(cassette.response.body_sha256.startsWith("sha256:")).toBe(true);
      expect(await bodySha256(cassette.response.body_path)).toBe(
        cassette.response.body_sha256,
      );
      expect(cassette.expected_failure_mode).toBeNull();
    }
  });

  it("basis-delta fixture preserves both FX bases", async () => {
    const fixtureFile = Bun.file(
      new URL("../../../../tests/fixtures/l5/fx/basis-delta-usd-gbp.json", import.meta.url),
    );
    const fixture = (await fixtureFile.json()) as FxBasisDelta;

    expect(fixture.customsSnapshot.basis).toBe("hmrc_customs_monthly");
    expect(fixture.commercialSnapshot.basis).toBe("frankfurter_commercial_spot");
    expect(fixture.basisDeltaBps).toBe(47);
  });
});

describe("hmrc customs FX", () => {
  it("L5-T-fx_engine-001 parses HMRC monthly CSV cassette without credentials", async () => {
    const result = await fetchHmrcMonthlySnapshot({
      baseCurrency: "USD",
      effectiveMonth: "2026-05",
      cassettePath: "hmrc-monthly-current-csv.json",
    });

    expect(result.ok).toBe(true);
    if (!result.ok) {
      throw new Error(result.message);
    }

    expect(result.snapshot).toEqual({
      basis: "hmrc_customs_monthly",
      effectiveMonth: "2026-05",
      baseCurrency: "USD",
      quoteCurrency: "GBP",
      rate: 0.7834,
      rateBps: 7834,
      sourceUrl:
        "https://www.trade-tariff.service.gov.uk/exchange_rates/monthly?year=2026",
      checkedAt: "2026-05-19T00:00:00Z",
      snapshotId: "hmrc-2026-05-USD-GBP-v1",
    });
    const cassette = await readCassette("hmrc-monthly-current-csv.json");
    expect(cassette.request.authenticated).toBe(false);
    expect(cassette.request.headers).not.toHaveProperty("authorization");
  });

  it("L5-T-fx_engine-005 plans rollover recalculation with old and new snapshot IDs", async () => {
    const previous = await expectHmrcSnapshot({
      baseCurrency: "USD",
      effectiveMonth: "2026-04",
      cassettePath: "hmrc-april-usd-gbp.json",
    });
    const fresh = await expectHmrcSnapshot({
      baseCurrency: "USD",
      effectiveMonth: "2026-05",
      cassettePath: "hmrc-monthly-current-csv.json",
    });

    const jobs = planHmrcRolloverRecalculation(
      previous,
      fresh,
      new Date("2026-05-02T08:00:00Z"),
    );

    expect(jobs).toEqual([
      {
        trigger: {
          kind: "monthly_rollover",
          previousMonth: "2026-04",
          newMonth: "2026-05",
        },
        previousSnapshotId: "hmrc-2026-04-USD-GBP-v1",
        newSnapshotId: "hmrc-2026-05-USD-GBP-v1",
        staleReason: "hmrc_monthly_rollover",
      },
    ]);
  });

  it("L5-T-fx_engine-006 plans correction recalculation while preserving the prior snapshot ID", async () => {
    const previous = await expectHmrcSnapshot({
      baseCurrency: "USD",
      effectiveMonth: "2026-05",
      cassettePath: "hmrc-monthly-current-csv.json",
    });
    const correctedResult = await fetchHmrcMonthlySnapshot({
      baseCurrency: "USD",
      effectiveMonth: "2026-05",
      cassettePath: "hmrc-may-corrected-usd-gbp.json",
      snapshotVersion: 2,
    });
    if (!correctedResult.ok) {
      throw new Error(correctedResult.message);
    }

    const jobs = planHmrcCorrectionRecalculation(
      previous,
      correctedResult.snapshot,
    );

    expect(correctedResult.snapshot.snapshotId).toBe("hmrc-2026-05-USD-GBP-v2");
    expect(jobs).toEqual([
      {
        trigger: {
          kind: "historical_correction",
          affectedMonth: "2026-05",
          previousSnapshotId: "hmrc-2026-05-USD-GBP-v1",
          newSnapshotId: "hmrc-2026-05-USD-GBP-v2",
        },
        previousSnapshotId: "hmrc-2026-05-USD-GBP-v1",
        newSnapshotId: "hmrc-2026-05-USD-GBP-v2",
        staleReason: "hmrc_historical_correction",
      },
    ]);
  });

  it("L5-T-fx_engine-007 emits FX_RATE_MISSING instead of fabricating HMRC rates", async () => {
    const missingMonth = await fetchHmrcMonthlySnapshot({
      baseCurrency: "USD",
      effectiveMonth: "2026-05",
      cassettePath: "hmrc-missing-month.json",
    });
    const malformedCsv = await fetchHmrcMonthlySnapshot({
      baseCurrency: "USD",
      effectiveMonth: "2026-05",
      cassettePath: "hmrc-malformed-csv.json",
    });
    const malformedXml = await fetchHmrcMonthlySnapshot({
      baseCurrency: "USD",
      effectiveMonth: "2026-05",
      cassettePath: "hmrc-malformed-xml.json",
    });

    expect(missingMonth).toMatchObject({
      ok: false,
      errorCode: "FX_RATE_MISSING",
      reason: "missing_month",
    });
    expect(malformedCsv).toMatchObject({
      ok: false,
      errorCode: "FX_RATE_MISSING",
      reason: "parse_failure",
    });
    expect(malformedXml).toMatchObject({
      ok: false,
      errorCode: "FX_RATE_MISSING",
      reason: "parse_failure",
    });
  });
});

describe("frankfurter commercial FX", () => {
  it("L5-T-fx_engine-002 parses Frankfurter rates.GBP without credentials", async () => {
    const result = await fetchFrankfurterCommercialSpot({
      baseCurrency: "USD",
      cassettePath: "frankfurter-usd-gbp.json",
    });

    expect(result.ok).toBe(true);
    if (!result.ok) {
      throw new Error(result.message);
    }

    expect(result.snapshot).toEqual({
      basis: "frankfurter_commercial_spot",
      effectiveDate: "2026-05-19",
      baseCurrency: "USD",
      quoteCurrency: "GBP",
      rate: 0.7871,
      rateBps: 7871,
      sourceUrl: "https://api.frankfurter.app/2026-05-19?from=USD&to=GBP",
      checkedAt: "2026-05-19T00:00:00Z",
      snapshotId: "frankfurter-2026-05-19-USD-GBP-v1",
    });
    expect(result.staleness).toBe("fresh");
    expect(result.commercialEstimateConfidenceBpsReduction).toBe(0);

    const missingGbp = await fetchFrankfurterCommercialSpot({
      baseCurrency: "USD",
      cassettePath: "frankfurter-missing-gbp.json",
    });
    const malformedJson = await fetchFrankfurterCommercialSpot({
      baseCurrency: "USD",
      cassettePath: "frankfurter-malformed-json.json",
    });
    expect(missingGbp).toMatchObject({
      ok: false,
      errorCode: "FX_RATE_MISSING",
      reason: "missing_gbp_rate",
      bidReadinessImpact: "blocker",
    });
    expect(malformedJson).toMatchObject({
      ok: false,
      errorCode: "FX_RATE_MISSING",
      reason: "parse_failure",
      bidReadinessImpact: "blocker",
    });

    const cassette = await readCassette("frankfurter-usd-gbp.json");
    expect(cassette.request.authenticated).toBe(false);
    expect(cassette.request.headers).not.toHaveProperty("authorization");
  });

  it("L5-T-fx_engine-008 keeps Frankfurter last-valid snapshots only inside TTL", async () => {
    const fetcher = createFrankfurterFetcher();
    const fresh = await fetcher.fetchSpotResult({
      baseCurrency: "USD",
      cassettePath: "frankfurter-usd-gbp.json",
    });
    if (!fresh.ok) {
      throw new Error(fresh.message);
    }

    const retained = await fetcher.fetchSpotResult({
      baseCurrency: "USD",
      cassettePath: "frankfurter-usd-gbp-429.json",
      maxAgeSeconds: 3600,
      now: new Date("2026-05-19T00:30:00Z"),
    });
    expect(retained.ok).toBe(true);
    if (!retained.ok) {
      throw new Error(retained.message);
    }
    expect(retained.snapshot.snapshotId).toBe(fresh.snapshot.snapshotId);
    expect(retained.staleness).toBe("last_valid");
    expect(retained.warning).toContain("[WARN] Frankfurter commercial FX returned HTTP 429");
    expect(retained.commercialEstimateConfidenceBpsReduction).toBeGreaterThan(0);

    const expired = await fetcher.fetchSpotResult({
      baseCurrency: "USD",
      cassettePath: "frankfurter-usd-gbp-429.json",
      maxAgeSeconds: 1,
      now: new Date("2026-05-19T00:30:00Z"),
    });
    expect(expired).toMatchObject({
      ok: false,
      errorCode: "FX_RATE_MISSING",
      reason: "last_valid_expired",
      bidReadinessImpact: "blocker",
    });
    if (!expired.ok) {
      expect(expired.warning).toContain("[WARN]");
      expect(expired.commercialEstimateConfidenceBpsReduction).toBeGreaterThan(0);
    }
  });

  it("L5-T-fx_engine-011 uses FRANKFURTER_BASE_URL and cassette fallback on transient outage", async () => {
    const probe = await probeFrankfurterPublicEndpoint({
      env: {
        FRANKFURTER_BASE_URL: "https://api.frankfurter.app/",
      },
      baseCurrency: "USD",
      cassettePath: "frankfurter-usd-gbp-500.json",
      fallbackCassettePath: "frankfurter-usd-gbp.json",
    });

    expect(probe.probeUrl).toBe(
      "https://api.frankfurter.app/latest?from=USD&to=GBP",
    );
    expect(probe.result.ok).toBe(true);
    if (!probe.result.ok) {
      throw new Error(probe.result.message);
    }

    expect(probe.result.snapshot.basis).toBe("frankfurter_commercial_spot");
    expect(probe.result.warning).toBe(
      "[WARN] Frankfurter commercial FX returned HTTP 500; used cassette fallback",
    );
    expect(probe.result.staleness).toBe("fresh");
  });
});

async function readCassette(fileName: string): Promise<CassetteFixture> {
  const cassetteFile = Bun.file(
    new URL(
      `../../../../tests/fixtures/cassettes/l5/fx/${fileName}`,
      import.meta.url,
    ),
  );
  return (await cassetteFile.json()) as CassetteFixture;
}

async function bodySha256(bodyPath: string): Promise<string> {
  const bodyFile = Bun.file(
    new URL(
      `../../../../tests/fixtures/cassettes/l5/fx/${bodyPath}`,
      import.meta.url,
    ),
  );
  const body = Buffer.from(await bodyFile.arrayBuffer());
  return `sha256:${createHash("sha256").update(body).digest("hex")}`;
}

async function expectHmrcSnapshot(opts: {
  baseCurrency: "USD" | "EUR" | "CNY";
  effectiveMonth: string;
  cassettePath: string;
}): Promise<FxSnapshot> {
  const result = await fetchHmrcMonthlySnapshot(opts);
  if (!result.ok) {
    throw new Error(result.message);
  }

  return result.snapshot;
}
