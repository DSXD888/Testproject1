import { describe, expect, it } from "bun:test";

import {
  type ProcurementProcessIdentity,
  type RelationResolutionInput,
  resolveProcessRelations,
} from "./resolve-process-relations";

const RelationScoringFixture = {
  sameOcid(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        canonicalOcid: "ocid-1",
        buyerIdentifier: "buyer-1",
        buyerName: "Example Borough Council",
        title: "Laptop supply framework",
        referenceNumber: "ABC-123",
        createdAt: "2026-03-01T00:00:00Z",
      },
      right: {
        id: "process-b",
        canonicalOcid: "ocid-1",
        buyerIdentifier: "buyer-1",
        buyerName: "Example Borough Council",
        title: "Laptop supply framework",
        referenceNumber: "ABC-123",
        createdAt: "2026-03-02T00:00:00Z",
      },
    });
  },

  linkedNoticeChain(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        canonicalOcid: "ocid-parent",
        buyerIdentifier: "buyer-1",
        title: "PPE framework refresh",
        relationHints: ["linked_notice_chain"],
        stage: "planning",
      },
      right: {
        id: "process-b",
        canonicalOcid: "ocid-child",
        buyerIdentifier: "buyer-1",
        title: "PPE framework refresh",
        stage: "tender",
        relatedOcidReferences: ["ocid-parent"],
      },
    });
  },

  frameworkCallOff(): RelationResolutionInput {
    return makePair({
      left: {
        id: "framework-process",
        buyerIdentifier: "buyer-1",
        buyerName: "Northshire NHS Trust",
        title: "Medical gloves framework",
        stage: "framework",
      },
      right: {
        id: "calloff-process",
        buyerIdentifier: "buyer-1",
        buyerName: "Northshire NHS Trust",
        title: "Medical gloves framework call-off",
        stage: "call_off",
      },
      heuristicOverrides: {
        totalScore: 0.84,
      },
    });
  },

  conflictingReferences(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        buyerIdentifier: "buyer-1",
        buyerName: "River City Council",
        title: "School furniture replacement",
        referenceNumber: "REF-111",
      },
      right: {
        id: "process-b",
        buyerIdentifier: "buyer-1",
        buyerName: "River City Council",
        title: "School furniture replacement",
        referenceNumber: "REF-222",
      },
      heuristicOverrides: {
        totalScore: 0.97,
      },
    });
  },

  differentLotStructure(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        buyerIdentifier: "buyer-1",
        buyerName: "River City Council",
        title: "School furniture replacement",
        lotCount: 1,
        estimatedValuePence: 100_000,
      },
      right: {
        id: "process-b",
        buyerIdentifier: "buyer-1",
        buyerName: "River City Council",
        title: "School furniture replacement",
        lotCount: 4,
        estimatedValuePence: 450_000,
      },
      heuristicOverrides: {
        totalScore: 0.6,
      },
    });
  },

  highScoreNoBlocker(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        title: "Emergency winter grit supply",
        buyerIdentifier: "buyer-a",
        referenceNumber: "GRIT-100",
      },
      right: {
        id: "process-b",
        title: "Bulk winter grit supply",
        buyerIdentifier: "buyer-b",
        referenceNumber: "GRIT-100",
      },
      heuristicOverrides: {
        totalScore: 0.96,
      },
    });
  },

  midScorePossibleDuplicate(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        title: "Portable cabins for temporary classrooms",
        buyerIdentifier: "buyer-a",
      },
      right: {
        id: "process-b",
        title: "Temporary modular office cabins",
        buyerIdentifier: "buyer-b",
      },
      heuristicOverrides: {
        totalScore: 0.84,
      },
    });
  },

  lowScore(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        title: "Catering services for council meetings",
        buyerIdentifier: "buyer-a",
      },
      right: {
        id: "process-b",
        title: "Salt bins and grit refill procurement",
        buyerIdentifier: "buyer-b",
      },
      heuristicOverrides: {
        totalScore: 0.7499,
      },
    });
  },

  evidenceReplay(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        title: "Emergency traffic sign replacement",
        buyerIdentifier: "buyer-a",
        cpvCodes: ["34992200"],
      },
      right: {
        id: "process-b",
        title: "Traffic sign replacement kit",
        buyerIdentifier: "buyer-b",
        cpvCodes: ["34992200"],
      },
      heuristicOverrides: {
        referenceMatch: 0.6,
        buyerMatch: 0,
        titleSimilarity: 0.8,
        cpvOverlap: 1,
        dateProximity: 0.9,
        valueProximity: 0.8,
      },
    });
  },

  transitiveRoots(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-b",
        canonicalProcessId: "process-a",
        canonicalCreatedAt: "2026-01-01T00:00:00Z",
        canonicalOcid: "ocid-merge",
        title: "Estate vehicles replacement",
      },
      right: {
        id: "process-c",
        canonicalProcessId: "process-c",
        canonicalCreatedAt: "2026-01-02T00:00:00Z",
        canonicalOcid: "ocid-merge",
        title: "Estate vehicles replacement",
      },
      aliases: [
        {
          aliasProcessId: "process-b",
          canonicalProcessId: "process-a",
          aliasReason: "merged_same_process",
        },
      ],
    });
  },

  lateMerge(withDecisions: boolean): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        canonicalOcid: "ocid-late-merge",
        title: "MRO consumables contract",
        pendingOperatorIntentCount: withDecisions ? 1 : 0,
      },
      right: {
        id: "process-b",
        canonicalOcid: "ocid-late-merge",
        title: "MRO consumables contract",
        operatorDecisionHeadCount: withDecisions ? 1 : 0,
      },
    });
  },

  clusterVisibility(): {
    related: RelationResolutionInput;
    duplicate: RelationResolutionInput;
  } {
    return {
      related: this.midScoreRelated(),
      duplicate: this.midScorePossibleDuplicate(),
    };
  },

  midScoreRelated(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        buyerIdentifier: "buyer-1",
        buyerName: "Harbour Authority",
        title: "Port safety equipment programme",
        stage: "planning",
      },
      right: {
        id: "process-b",
        buyerIdentifier: "buyer-1",
        buyerName: "Harbour Authority",
        title: "Port safety equipment programme",
        stage: "tender",
      },
      heuristicOverrides: {
        totalScore: 0.82,
      },
    });
  },

  sameOcidBuyerMismatch(): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        canonicalOcid: "ocid-same",
        buyerIdentifier: "buyer-1",
        title: "Fire station breathing apparatus",
      },
      right: {
        id: "process-b",
        canonicalOcid: "ocid-same",
        buyerIdentifier: "buyer-2",
        title: "Fire station breathing apparatus",
      },
      heuristicOverrides: {
        totalScore: 0.99,
      },
    });
  },

  thresholdBoundaries(score: number): RelationResolutionInput {
    return makePair({
      left: {
        id: "process-a",
        title: "Mixed office consumables",
        buyerIdentifier: "buyer-a",
      },
      right: {
        id: "process-b",
        title: "Mixed office consumables",
        buyerIdentifier: "buyer-b",
      },
      heuristicOverrides: {
        totalScore: score,
      },
    });
  },
} as const;

describe("resolveProcessRelations", () => {
  it("L3-T-resolve_process_relations-001: same OCID pairs merge without heuristic fallback", () => {
    const result = resolveProcessRelations(RelationScoringFixture.sameOcid());

    expect(result.classification).toBe("merged_same_process");
    expect(result.evidence.exactMatchRule).toBe("same_ocid");
    expect(result.evidence.heuristicUsed).toBe(false);
  });

  it("L3-T-resolve_process_relations-002: linked notice chains merge as one process", () => {
    const result = resolveProcessRelations(RelationScoringFixture.linkedNoticeChain());

    expect(result.classification).toBe("merged_same_process");
    expect(result.evidence.exactMatchRule).toBe("linked_notice_chain");
    expect(result.aliasRedirect?.canonicalProcessId).toBe(result.canonicalProcessId);
  });

  it("L3-T-resolve_process_relations-003: framework/call-off relations stay related but unmerged", () => {
    const result = resolveProcessRelations(RelationScoringFixture.frameworkCallOff());

    expect(result.classification).toBe("related_not_merged");
    expect(result.relationEdge?.writeDisposition).toBe("required");
    expect(result.canonicalUpdates).toHaveLength(0);
  });

  it("L3-T-resolve_process_relations-004: conflicting references hard-block merge even with high similarity", () => {
    const result = resolveProcessRelations(RelationScoringFixture.conflictingReferences());

    expect(result.classification).not.toBe("merged_same_process");
    expect(result.evidence.blockers).toContain("reference_conflict");
  });

  it("L3-T-resolve_process_relations-005: incompatible lot/value structures classify as separate and write no edge", () => {
    const result = resolveProcessRelations(RelationScoringFixture.differentLotStructure());

    expect(result.classification).toBe("separate");
    expect(result.evidence.blockers).toContain("lot_or_value_structure_conflict");
    expect(result.relationEdge).toBeNull();
  });

  it("L3-T-resolve_process_relations-006: high scores without blockers merge automatically", () => {
    const result = resolveProcessRelations(RelationScoringFixture.highScoreNoBlocker());

    expect(result.classification).toBe("merged_same_process");
    expect(result.evidence.thresholdBucket).toBe("merge_candidate");
  });

  it("L3-T-resolve_process_relations-007: ambiguous mid-score pairs stay possible_duplicate and keep a relation edge", () => {
    const result = resolveProcessRelations(
      RelationScoringFixture.midScorePossibleDuplicate(),
    );

    expect(result.classification).toBe("possible_duplicate");
    expect(result.operatorReviewRequired).toBe(true);
    expect(result.relationEdge?.relationType).toBe("possible_duplicate");
  });

  it("L3-T-resolve_process_relations-008: scores below 0.75 stay separate and write no row", () => {
    const result = resolveProcessRelations(RelationScoringFixture.lowScore());

    expect(result.classification).toBe("separate");
    expect(result.evidence.thresholdBucket).toBe("below_review");
    expect(result.relationEdge).toBeNull();
  });

  it("L3-T-resolve_process_relations-009: evidence payloads replay deterministically", () => {
    const fixture = RelationScoringFixture.evidenceReplay();

    const first = resolveProcessRelations(fixture);
    const second = resolveProcessRelations(fixture);

    expect(first.classification).toBe(second.classification);
    expect(first.evidence).toEqual(second.evidence);
    expect(first.evidence.relationModelVersion).toBe(1);
  });

  it("L3-T-resolve_process_relations-010: overlapping merges resolve roots first and pick a deterministic survivor", () => {
    const earlierRoot = resolveProcessRelations(RelationScoringFixture.transitiveRoots());
    const tiedRoots = resolveProcessRelations(
      makePair({
        left: {
          id: "process-x",
          canonicalProcessId: "process-b",
          canonicalCreatedAt: "2026-01-01T00:00:00Z",
          canonicalOcid: "ocid-merge",
          title: "Fleet replacement",
        },
        right: {
          id: "process-y",
          canonicalProcessId: "process-a",
          canonicalCreatedAt: "2026-01-01T00:00:00Z",
          canonicalOcid: "ocid-merge",
          title: "Fleet replacement",
        },
      }),
    );

    expect(earlierRoot.canonicalProcessId).toBe("process-a");
    expect(earlierRoot.retiredProcessId).toBe("process-c");
    expect(tiedRoots.canonicalProcessId).toBe("process-a");
  });

  it("L3-T-resolve_process_relations-011: late merges stale decisions only when decision state exists and always trigger recompute", () => {
    const withDecisions = resolveProcessRelations(
      RelationScoringFixture.lateMerge(true),
    );
    const withoutDecisions = resolveProcessRelations(
      RelationScoringFixture.lateMerge(false),
    );

    expect(withDecisions.lateMergeEffect).toEqual({
      manifestRegenerationRequired: true,
      pipelineAssessmentRecomputeRequired: true,
      decisionStale: true,
      stalePendingOperatorIntents: true,
      staleOperatorDecisionHeads: true,
      emitIdentityChangedNotification: true,
    });
    expect(withoutDecisions.lateMergeEffect).toEqual({
      manifestRegenerationRequired: true,
      pipelineAssessmentRecomputeRequired: true,
      decisionStale: false,
      stalePendingOperatorIntents: false,
      staleOperatorDecisionHeads: false,
      emitIdentityChangedNotification: false,
    });
  });

  it("L3-T-resolve_process_relations-012: visible non-merge clusters stay trust-aware and avoid destructive auto-merge", () => {
    const fixtures = RelationScoringFixture.clusterVisibility();
    const related = resolveProcessRelations(fixtures.related);
    const duplicate = resolveProcessRelations(fixtures.duplicate);

    expect(related.clusterView).toMatchObject({
      visible: true,
      relationType: "related_not_merged",
      cachePolicy: "trust_aware",
    });
    expect(duplicate.clusterView).toMatchObject({
      visible: true,
      relationType: "possible_duplicate",
      cachePolicy: "trust_aware",
    });
    expect(related.aliasRedirect).toBeNull();
    expect(duplicate.aliasRedirect).toBeNull();
  });

  it("L3-T-resolve_process_relations-013: planning/tender mid-score pairs classify as related_not_merged", () => {
    const result = resolveProcessRelations(RelationScoringFixture.midScoreRelated());

    expect(result.classification).toBe("related_not_merged");
    expect(result.evidence.exactMatchRule).toBe("planning_tender");
    expect(result.relationEdge?.relationType).toBe("related_not_merged");
  });

  it("L3-T-resolve_process_relations-014: same-OCID buyer mismatch stays possible_duplicate and records the blocker", () => {
    const result = resolveProcessRelations(
      RelationScoringFixture.sameOcidBuyerMismatch(),
    );

    expect(result.classification).toBe("possible_duplicate");
    expect(result.evidence.exactMatchRule).toBe("same_ocid");
    expect(result.evidence.blockers).toContain("buyer_mismatch");
  });

  it("L3-T-resolve_process_relations-015: threshold boundaries stay exact at 0.9499 and 0.9500", () => {
    const duplicate = resolveProcessRelations(
      RelationScoringFixture.thresholdBoundaries(0.9499),
    );
    const merged = resolveProcessRelations(
      RelationScoringFixture.thresholdBoundaries(0.95),
    );

    expect(duplicate.classification).toBe("possible_duplicate");
    expect(duplicate.evidence.thresholdBucket).toBe("review_candidate");
    expect(merged.classification).toBe("merged_same_process");
    expect(merged.evidence.thresholdBucket).toBe("merge_candidate");
  });
});

function makePair(input: {
  left: Partial<ProcurementProcessIdentity>;
  right: Partial<ProcurementProcessIdentity>;
  aliases?: RelationResolutionInput["aliases"];
  heuristicOverrides?: RelationResolutionInput["heuristicOverrides"];
}): RelationResolutionInput {
  return {
    left: makeProcess("process-left", input.left),
    right: makeProcess("process-right", input.right),
    aliases: input.aliases,
    heuristicOverrides: input.heuristicOverrides,
  };
}

function makeProcess(
  defaultId: string,
  overrides: Partial<ProcurementProcessIdentity>,
): ProcurementProcessIdentity {
  const id = overrides.id ?? defaultId;

  return {
    id,
    canonicalProcessId: overrides.canonicalProcessId ?? id,
    canonicalCreatedAt: overrides.canonicalCreatedAt ?? overrides.createdAt ?? "2026-03-01T00:00:00Z",
    canonicalOcid: overrides.canonicalOcid ?? null,
    createdAt: overrides.createdAt ?? "2026-03-01T00:00:00Z",
    buyerName: overrides.buyerName ?? null,
    buyerIdentifier: overrides.buyerIdentifier ?? null,
    title: overrides.title ?? `Title for ${id}`,
    referenceNumber: overrides.referenceNumber ?? null,
    publishedAt: overrides.publishedAt ?? "2026-03-02T00:00:00Z",
    estimatedValuePence: overrides.estimatedValuePence ?? 100_000,
    lotCount: overrides.lotCount ?? 1,
    cpvCodes: overrides.cpvCodes ?? ["30000000"],
    stage: overrides.stage ?? "tender",
    relationHints: overrides.relationHints ?? [],
    relatedProcessIds: overrides.relatedProcessIds ?? [],
    relatedOcidReferences: overrides.relatedOcidReferences ?? [],
    explicitSeparateProcess: overrides.explicitSeparateProcess ?? false,
    pendingOperatorIntentCount: overrides.pendingOperatorIntentCount ?? 0,
    operatorDecisionHeadCount: overrides.operatorDecisionHeadCount ?? 0,
  };
}
