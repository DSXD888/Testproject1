import { describe, expect, it } from "bun:test";
import { createHash } from "node:crypto";

import type {
  FxBasisDelta,
  FxRecalculationTrigger,
  FxSnapshot,
} from "./fx-types";

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
