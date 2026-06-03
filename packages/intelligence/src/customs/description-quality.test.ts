import { describe, expect, it } from "bun:test";

import { gateDescriptionQuality } from "./description-quality";

describe("description quality service gate", () => {
  it("TS-DQ-001 identifies supply-and-install shelving as goods-led mixed", () => {
    const verdict = gateDescriptionQuality({
      text: "Supply and installation of steel shelving",
    });

    expect(verdict).toMatchObject({
      kind: "goods_led_mixed",
      goodsHeadNoun: "shelving",
      supplySignal: "supply",
    });
    expect(verdict.kind).toBe("goods_led_mixed");
    if (verdict.kind === "goods_led_mixed") {
      expect(verdict.installationTerms).toContain("installation");
    }
  });

  it("TS-DQ-002 accepts maintenance kits as physical goods", () => {
    const verdict = gateDescriptionQuality({
      text: "Supply of maintenance kits",
    });

    expect(verdict.kind).toBe("accept");
    if (verdict.kind === "accept") {
      expect(verdict.attributes.headNoun).toBe("kits");
    }
  });

  it("TS-DQ-003 identifies festive lighting install and maintain as goods-led mixed", () => {
    const verdict = gateDescriptionQuality({
      text: "Supply, install and maintain festive lighting",
    });

    expect(verdict).toMatchObject({
      kind: "goods_led_mixed",
      goodsHeadNoun: "lighting",
      supplySignal: "supply",
    });
    expect(verdict.kind).toBe("goods_led_mixed");
    if (verdict.kind === "goods_led_mixed") {
      expect(verdict.installationTerms).toContain("install");
      expect(verdict.installationTerms).toContain("maintain");
    }
  });

  it("TS-DQ-004 rejects installation services", () => {
    const verdict = gateDescriptionQuality({
      text: "Installation services",
    });

    expect(verdict).toEqual({
      kind: "reject_service",
      reason: "service_description_not_physical_goods",
    });
  });

  it("TS-DQ-005 rejects grounds maintenance and grass cutting", () => {
    const verdict = gateDescriptionQuality({
      text: "Provision of grounds maintenance and grass cutting",
    });

    expect(verdict).toEqual({
      kind: "reject_service",
      reason: "service_description_not_physical_goods",
    });
  });

  it("TS-DQ-006 rejects managed cleaning with supplied materials", () => {
    const verdict = gateDescriptionQuality({
      text: "Managed cleaning service; contractor supplies materials",
    });

    expect(verdict).toEqual({
      kind: "reject_service",
      reason: "service_description_not_physical_goods",
    });
  });

  it("TS-DQ-007 rejects estate electrical maintenance", () => {
    const verdict = gateDescriptionQuality({
      text: "Reactive and planned electrical maintenance across estate",
    });

    expect(verdict).toEqual({
      kind: "reject_service",
      reason: "service_description_not_physical_goods",
    });
  });

  it("TS-DQ-008 rejects consultancy and training", () => {
    const verdict = gateDescriptionQuality({
      text: "Consultancy and training",
    });

    expect(verdict).toEqual({
      kind: "reject_service",
      reason: "service_description_not_physical_goods",
    });
  });
});
