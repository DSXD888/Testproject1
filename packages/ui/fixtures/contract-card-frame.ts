import { createElement } from "react";

import type { ContractCardFrameProps } from "../contract-card-frame";

export const contractCardFrameFixture: ContractCardFrameProps = {
  summary: {
    processId: "process-123",
    title: "Emergency Medical Consumables",
    buyerName: "NHS Example Trust",
    publishedDate: "2026-03-28",
    deadlineDate: "2026-04-21",
    estimatedValuePence: 245_000_00,
    lotCount: 3,
    accessRoute: "open_tender",
    legalCategory: "goods",
  },
  metrics: {
    pipelineState: "error",
    pricedCoverageBps: 6700,
    netQualifiedCoverageBps: 5400,
    netQualifiedValueCoverageBps: 4900,
    grossMarginBps: 3250,
    netMarginBaseBps: 2110,
    candidateCount: 8,
    quotedCandidateCount: 5,
  },
  decision: {
    disposition: "watch",
    isStale: true,
    decidedAt: "2026-03-28T10:00:00Z",
  },
  rightRail: createElement("span", { "data-slot": "right-rail" }, "Fresh"),
  actions: createElement("button", { type: "button", "data-slot": "actions" }, "Review"),
};
