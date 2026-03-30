import { describe, expect, test } from "bun:test";

import {
  renderErrorManifest,
  type ErrorManifestInput
} from "../render-error-manifest.ts";

const BASE_INPUT: ErrorManifestInput = {
  id: "error-1",
  jobId: "job-1",
  processId: "process-1",
  pipelineStage: "source-health",
  workflowStep: "fts-canary",
  errorCode: "AGENT_SKILL_FAILED",
  errorMessage: "Known query returned no usable extraction result.",
  errorContext: {
    source: "fts",
    query: "nitrile gloves",
    observedItemCount: 0
  },
  errorChain: [
    {
      code: "AGENT_RUN_FAILED",
      message: "The canary session completed with a non-zero exit status."
    },
    {
      code: "AGENT_SKILL_FAILED",
      message: "Known query returned no usable extraction result."
    }
  ],
  skillPath: "skills/source-health/fts",
  agentSessionId: "session-1",
  runtimeContext: {
    cliVendor: "codex",
    cliVersion: "5.4",
    adapterVersion: "adapter-1",
    modelId: "gpt-5.4",
    configHash: "cfg-123",
    skillVersion: "2026-03-30",
    pid: 101,
    pgid: 202,
    cleanupVerdict: "Clean",
    lastValidCheckpointSequence: 7,
    daemonInstanceId: "daemon-1",
    commitSeq: 42
  },
  markdownPath: "artifacts/errors/error-1.md",
  sessionArtifactPath: "artifacts/sessions/session-1",
  linkedArtifacts: [
    {
      label: "control-events",
      path: "artifacts/traces/control-events.jsonl"
    }
  ],
  nextDebuggingAction:
    "Replay the FTS canary and compare the current fingerprint with the last passing run.",
  resolved: false,
  createdAt: "2026-03-30T14:00:00Z"
};

const cloneInput = (): ErrorManifestInput => structuredClone(BASE_INPUT);

describe("renderErrorManifest", () => {
  test("L3-T-render_error_manifest-001 keeps the stable section order and required fields", () => {
    const output = renderErrorManifest(cloneInput());

    expect(output.sections).toEqual([
      "error-chain",
      "job-process-session-context",
      "runtime-context",
      "error-context",
      "linked-artifacts",
      "next-debugging-action",
      "resolution"
    ]);

    const headingOrder = [
      "## Error Chain",
      "## Job / Process / Session Context",
      "## Runtime Context",
      "## Error Context",
      "## Linked Artifacts",
      "## Next Debugging Action",
      "## Resolution"
    ];

    let previousIndex = -1;
    for (const heading of headingOrder) {
      const index = output.markdown.indexOf(heading);
      expect(index).toBeGreaterThan(previousIndex);
      previousIndex = index;
    }

    expect(output.markdown).toContain("- job_id: job-1");
    expect(output.markdown).toContain("- process_id: process-1");
    expect(output.markdown).toContain("- agent_session_id: session-1");
    expect(output.markdown).toContain("1. AGENT_RUN_FAILED:");
    expect(output.markdown).toContain("2. AGENT_SKILL_FAILED:");
    expect(output.markdown).toContain("- markdown_path: artifacts/errors/error-1.md");
    expect(output.markdown).toContain("## Next Debugging Action");
    expect(output.markdown).toContain("Replay the FTS canary");
    expect(output.markdown).toContain("- resolved: false");
  });

  test("L3-T-render_error_manifest-002 maps logical source-health classes onto existing ErrorCode strings", () => {
    const input = cloneInput();
    input.errorCode = "SOURCE_LAYOUT_CHANGED";
    input.logicalSourceHealthClass = "SOURCE_LAYOUT_CHANGED";
    input.errorMessage = "Page fingerprint drifted on the FTS search results.";
    input.errorContext = {
      source: "fts",
      expectedFingerprint: "a1b2",
      observedFingerprint: "c3d4"
    };

    const output = renderErrorManifest(input);

    expect(output.errorCode).toBe("PAGE_SHAPE_DRIFT");
    expect(output.markdown).toContain("- error_code: PAGE_SHAPE_DRIFT");
    expect(output.markdown).toContain(
      "\"logicalSourceHealthClass\": \"SOURCE_LAYOUT_CHANGED\""
    );
    expect(output.markdown).not.toContain("- error_code: SOURCE_LAYOUT_CHANGED");
  });

  test("L3-T-render_error_manifest-003 preserves failure details when the manifest is resolved", () => {
    const unresolved = renderErrorManifest(cloneInput());
    const resolvedInput = cloneInput();
    resolvedInput.resolved = true;
    resolvedInput.resolvedAt = "2026-03-30T15:00:00Z";
    resolvedInput.resolvedBy = "operator@example.com";

    const resolved = renderErrorManifest(resolvedInput);

    expect(unresolved.markdown).toContain(
      "Known query returned no usable extraction result."
    );
    expect(resolved.markdown).toContain(
      "Known query returned no usable extraction result."
    );
    expect(resolved.markdown).toContain("- markdown_path: artifacts/errors/error-1.md");
    expect(resolved.markdown).toContain("\"observedItemCount\": 0");
    expect(resolved.markdown).toContain("- resolved: true");
    expect(resolved.markdown).toContain("- resolved_at: 2026-03-30T15:00:00Z");
    expect(resolved.markdown).toContain("- resolved_by: operator@example.com");
  });

  test("L3-T-render_error_manifest-004 escapes hostile inline content and fences structured context", () => {
    const alertCall = `${["al", "ert"].join("")}(1)`;
    const secondAlertCall = `${["al", "ert"].join("")}(2)`;
    const hostileScript = `<scr` + `ipt>${alertCall}</scr` + `ipt>`;
    const hostileLink = `[click](javascript:${alertCall})`;
    const hostileImage = `<img src=x onerror=${alertCall}>`;
    const hostileOpenLink = `[open](javascript:${secondAlertCall})`;
    const input = cloneInput();
    input.errorMessage = `${hostileScript} ${hostileLink}`;
    input.nextDebuggingAction = `${hostileImage} ${hostileOpenLink}`;
    input.errorContext = {
      hostileHtml: hostileScript,
      hostileLink
    };

    const output = renderErrorManifest(input);

    expect(output.markdown).toContain(
      "&lt;script&gt;alert\\(1\\)&lt;/script&gt; \\[click\\]\\(javascript:alert\\(1\\)\\)"
    );
    expect(output.markdown).toContain(
      "&lt;img src=x onerror=alert\\(1\\)&gt; \\[open\\]\\(javascript:alert\\(2\\)\\)"
    );
    expect(output.markdown).toContain("```json");
    expect(output.markdown).toContain("\\u003cscript\\u003ealert(1)\\u003c/script\\u003e");
    expect(output.markdown).not.toContain(hostileScript);
    expect(output.markdown).not.toContain(hostileImage);
  });
});
