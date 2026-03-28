import { describe, expect, it } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import { ContractCardFrame } from "../contract-card-frame";
import { contractCardFrameFixture } from "../fixtures/contract-card-frame";
import { freshnessBadgeFixture } from "../fixtures/freshness-badge";
import { pipelineBadgeFixture } from "../fixtures/pipeline-badge";
import { FreshnessBadge } from "../freshness-badge";
import { PipelineBadge } from "../pipeline-badge";

describe("packages/ui shared components", () => {
  it("TS-UI-001: shared UI components stay presentational and backend-agnostic", async () => {
    const files = [
      "../contract-card-frame.tsx",
      "../pipeline-badge.tsx",
      "../freshness-badge.tsx",
    ];
    const bannedPatterns = [
      "QueryService",
      "Supabase",
      "supabase",
      "invoke(",
      "src-tauri",
      "daemon",
      "sqlite",
      "websocket",
      "fetch(",
      "http://",
      "https://",
    ];

    for (const relativePath of files) {
      const fileText = await Bun.file(new URL(relativePath, import.meta.url)).text();
      for (const bannedPattern of bannedPatterns) {
        expect(fileText.includes(bannedPattern)).toBe(false);
      }
    }
  });

  it("TS-UI-002: ContractCardFrame renders summary, metrics, decision, rightRail, and actions with error state support", () => {
    const markup = renderToStaticMarkup(<ContractCardFrame {...contractCardFrameFixture} />);

    expect(markup).toContain("Emergency Medical Consumables");
    expect(markup).toContain("NHS Example Trust");
    expect(markup).toContain("GBP 245,000.00");
    expect(markup).toContain("5 quoted / 8 total");
    expect(markup).toContain('data-slot="right-rail"');
    expect(markup).toContain('data-slot="actions"');
    expect(markup).toContain('data-state="error"');
    expect(markup).toContain("Decision may be outdated");
  });

  it("TS-UI-004: PipelineBadge always renders text plus icon plus color and hides unverified validation state", () => {
    const validatedMarkup = renderToStaticMarkup(<PipelineBadge {...pipelineBadgeFixture} />);
    const unverifiedMarkup = renderToStaticMarkup(
      <PipelineBadge state="qualified" validationState="unverified" size="md" />,
    );

    expect(validatedMarkup).toContain("Qualified");
    expect(validatedMarkup).toContain('pipeline-badge__icon');
    expect(validatedMarkup).toContain('data-tone="good"');
    expect(validatedMarkup).toContain("Validated");
    expect(unverifiedMarkup).not.toContain("Unverified");
    expect(unverifiedMarkup).not.toContain("pipeline-badge__validation");
  });

  it("TS-UI-005: FreshnessBadge renders only from the supplied VM shape", () => {
    const markup = renderToStaticMarkup(<FreshnessBadge {...freshnessBadgeFixture} />);

    expect(markup).toContain("2 min ago");
    expect(markup).toContain('data-tone="warn"');
    expect(markup).toContain("Last sync: 2026-03-28T10:05:23Z");
    expect(markup).toContain("freshness-badge__icon");
  });

  it("TS-UI-008: fixture-driven tests stay backend-agnostic", async () => {
    const fixtureFiles = [
      "../fixtures/contract-card-frame.ts",
      "../fixtures/pipeline-badge.ts",
      "../fixtures/freshness-badge.ts",
    ];

    for (const relativePath of fixtureFiles) {
      const fileText = await Bun.file(new URL(relativePath, import.meta.url)).text();
      expect(fileText.includes("fetch(")).toBe(false);
      expect(fileText.includes("sqlite")).toBe(false);
      expect(fileText.includes("Supabase")).toBe(false);
      expect(fileText.includes("daemon")).toBe(false);
    }
  });
});
