import {
  ERROR_MANIFEST_SECTIONS,
  renderErrorMarkdown
} from "./error.md.ts";

export const EXISTING_ERROR_CODES = [
  "SOURCE_CHANGED",
  "NOTICE_PARSE_FAILED",
  "GOODS_CLASSIFICATION_LOW_CONFIDENCE",
  "SUPPLIER_MATCH_LOW_CONFIDENCE",
  "COMMODITY_CODE_LOW_CONFIDENCE",
  "REVENUE_BASIS_UNCERTAIN",
  "FPO_ACCESS_UNAVAILABLE",
  "AGENT_STALLED",
  "AGENT_AUTH_CHALLENGE",
  "AGENT_SKILL_FAILED",
  "AGENT_OUTPUT_CONTRACT_INVALID",
  "AGENT_SUSPECT_OUTPUT",
  "AGENT_PARTIAL_FAILURE",
  "AGENT_INTERNAL_ERROR",
  "AGENT_RUN_FAILED",
  "AGENT_CLEANUP_INCOMPLETE",
  "AGENT_DAEMON_CRASH_DURING_SESSION",
  "AGENT_PROTOCOL_MISMATCH",
  "SOURCE_SOFT_BLOCKED",
  "SOURCE_CAPTCHA_BLOCKED",
  "AUTH_STATE_STALE",
  "PAGE_SHAPE_DRIFT",
  "LOCALE_OR_CURRENCY_DRIFT",
  "HOST_SLEEP_INTERRUPTED",
  "HOST_RESOURCE_EXHAUSTED",
  "SKILL_DIRECTORY_INVALID",
  "SQLITE_WRITE_FAILED",
  "DAEMON_STARTUP_FAILED"
] as const;

export type ErrorCode = (typeof EXISTING_ERROR_CODES)[number];

export const SOURCE_HEALTH_ERROR_CODE_MAP = {
  SOURCE_UNREACHABLE: "AGENT_RUN_FAILED",
  SOURCE_LAYOUT_CHANGED: "PAGE_SHAPE_DRIFT",
  SOURCE_LOCALE_DRIFT: "LOCALE_OR_CURRENCY_DRIFT",
  AGENT_AUTH_CHALLENGE: "AGENT_AUTH_CHALLENGE",
  SOURCE_BLOCKED: "SOURCE_SOFT_BLOCKED",
  SOURCE_EXTRACTION_FAILED: "AGENT_SKILL_FAILED"
} as const satisfies Record<string, ErrorCode>;

export type LogicalSourceHealthClass = keyof typeof SOURCE_HEALTH_ERROR_CODE_MAP;

export interface ErrorRuntimeContext {
  cliVendor?: string | null;
  cliVersion?: string | null;
  adapterVersion?: string | null;
  modelId?: string | null;
  configHash?: string | null;
  skillVersion?: string | null;
  pid?: number | null;
  pgid?: number | null;
  cleanupVerdict?: string | null;
  lastValidCheckpointSequence?: number | null;
  daemonInstanceId?: string | null;
  commitSeq?: number | null;
}

export interface ErrorChainEntry {
  code?: string | null;
  message: string;
  detail?: string | null;
}

export interface LinkedArtifact {
  label: string;
  path: string;
}

export interface ErrorManifestInput {
  id: string;
  jobId?: string | null;
  processId?: string | null;
  pipelineStage: string;
  workflowStep: string;
  errorCode?: string | null;
  logicalSourceHealthClass?: string | null;
  errorMessage: string;
  errorContext: unknown;
  errorChain?: ErrorChainEntry[] | null;
  skillPath?: string | null;
  agentSessionId?: string | null;
  runtimeContext?: ErrorRuntimeContext | null;
  markdownPath: string;
  sessionArtifactPath?: string | null;
  linkedArtifacts?: LinkedArtifact[] | null;
  nextDebuggingAction: string;
  resolved: boolean;
  resolvedAt?: string | null;
  resolvedBy?: string | null;
  createdAt: string;
}

export interface ErrorManifestOutput {
  markdown: string;
  sections: string[];
  errorCode: string;
  logicalSourceHealthClass?: string;
  resolved: boolean;
  markdownPath: string;
}

type NormalizedErrorManifest = {
  errorCode: string;
  logicalSourceHealthClass?: string;
  errorChain: ErrorChainEntry[];
  errorContext: unknown;
  linkedArtifacts: LinkedArtifact[];
  runtimeContext: ErrorRuntimeContext;
};

const EXISTING_ERROR_CODE_SET = new Set<string>(EXISTING_ERROR_CODES);

export const renderErrorManifest = (
  input: ErrorManifestInput
): ErrorManifestOutput => {
  const normalized = normalizeManifest(input);
  const sections = ERROR_MANIFEST_SECTIONS.map((section) => section.id);
  const markdown = renderErrorMarkdown(
    new Map<string, string>([
      ["error-chain", renderErrorChainSection(input, normalized)],
      [
        "job-process-session-context",
        renderExecutionContextSection(input)
      ],
      ["runtime-context", renderRuntimeContextSection(normalized.runtimeContext)],
      ["error-context", renderErrorContextSection(normalized.errorContext)],
      ["linked-artifacts", renderLinkedArtifactsSection(normalized.linkedArtifacts)],
      [
        "next-debugging-action",
        renderNextDebuggingActionSection(input.nextDebuggingAction)
      ],
      ["resolution", renderResolutionSection(input)]
    ])
  );

  return {
    markdown,
    sections,
    errorCode: normalized.errorCode,
    logicalSourceHealthClass: normalized.logicalSourceHealthClass,
    resolved: input.resolved,
    markdownPath: input.markdownPath
  };
};

const normalizeManifest = (
  input: ErrorManifestInput
): NormalizedErrorManifest => {
  const logicalSourceHealthClass =
    normalizeLogicalSourceHealthClass(input.logicalSourceHealthClass) ??
    normalizeLogicalSourceHealthClass(input.errorCode);
  const errorCode = resolveErrorCode(input.errorCode, logicalSourceHealthClass);
  const errorContext = normalizeErrorContext(
    input.errorContext,
    logicalSourceHealthClass
  );

  return {
    errorCode,
    logicalSourceHealthClass,
    errorChain: normalizeErrorChain(
      input.errorChain ?? [],
      errorCode,
      input.errorMessage
    ),
    errorContext,
    linkedArtifacts: normalizeLinkedArtifacts(input),
    runtimeContext: input.runtimeContext ?? {}
  };
};

const resolveErrorCode = (
  rawErrorCode: string | null | undefined,
  logicalSourceHealthClass?: string
): string => {
  const raw = normalizeString(rawErrorCode);
  if (raw && EXISTING_ERROR_CODE_SET.has(raw)) {
    return raw;
  }

  const mappedRaw = raw ? SOURCE_HEALTH_ERROR_CODE_MAP[raw as LogicalSourceHealthClass] : undefined;
  if (mappedRaw) {
    return mappedRaw;
  }

  const mappedLogical = logicalSourceHealthClass
    ? SOURCE_HEALTH_ERROR_CODE_MAP[
        logicalSourceHealthClass as LogicalSourceHealthClass
      ]
    : undefined;
  if (mappedLogical) {
    return mappedLogical;
  }

  return raw ?? "AGENT_INTERNAL_ERROR";
};

const normalizeLogicalSourceHealthClass = (
  value: string | null | undefined
): string | undefined => {
  const normalized = normalizeString(value);
  if (!normalized) {
    return undefined;
  }

  return normalized in SOURCE_HEALTH_ERROR_CODE_MAP ? normalized : undefined;
};

const normalizeErrorChain = (
  errorChain: readonly ErrorChainEntry[],
  errorCode: string,
  errorMessage: string
): ErrorChainEntry[] => {
  if (errorChain.length > 0) {
    return errorChain.map((entry) => ({
      code: normalizeString(entry.code) ?? undefined,
      message: entry.message,
      detail: normalizeString(entry.detail) ?? undefined
    }));
  }

  return [{ code: errorCode, message: errorMessage }];
};

const normalizeErrorContext = (
  errorContext: unknown,
  logicalSourceHealthClass?: string
): unknown => {
  if (!logicalSourceHealthClass) {
    return errorContext;
  }

  if (isPlainObject(errorContext)) {
    if ("logicalSourceHealthClass" in errorContext) {
      return errorContext;
    }
    return {
      logicalSourceHealthClass,
      ...errorContext
    };
  }

  if (errorContext === undefined) {
    return { logicalSourceHealthClass };
  }

  return {
    logicalSourceHealthClass,
    value: errorContext
  };
};

const normalizeLinkedArtifacts = (
  input: ErrorManifestInput
): LinkedArtifact[] => {
  const linkedArtifacts: LinkedArtifact[] = [
    { label: "markdown_path", path: input.markdownPath }
  ];

  const sessionArtifactPath = normalizeString(input.sessionArtifactPath);
  if (sessionArtifactPath) {
    linkedArtifacts.push({
      label: "session_artifact_path",
      path: sessionArtifactPath
    });
  }

  for (const artifact of input.linkedArtifacts ?? []) {
    const label = normalizeString(artifact.label);
    const path = normalizeString(artifact.path);
    if (!label || !path) {
      continue;
    }
    if (
      linkedArtifacts.some(
        (candidate) => candidate.label === label && candidate.path === path
      )
    ) {
      continue;
    }
    linkedArtifacts.push({ label, path });
  }

  return linkedArtifacts;
};

const renderErrorChainSection = (
  input: ErrorManifestInput,
  normalized: NormalizedErrorManifest
): string => {
  const lines = [
    `- error_code: ${escapeMarkdownText(normalized.errorCode)}`,
    `- logical_source_health_class: ${escapeMarkdownText(
      normalized.logicalSourceHealthClass ?? "not recorded"
    )}`,
    `- error_message: ${escapeMarkdownText(input.errorMessage)}`,
    `- created_at: ${escapeMarkdownText(input.createdAt)}`,
    "",
    "### Chain"
  ];

  normalized.errorChain.forEach((entry, index) => {
    const code = escapeMarkdownText(entry.code ?? normalized.errorCode);
    const message = escapeMarkdownText(entry.message);
    const detail = normalizeString(entry.detail);
    lines.push(`${index + 1}. ${code}: ${message}`);
    if (detail) {
      lines.push(`   detail: ${escapeMarkdownText(detail)}`);
    }
  });

  return lines.join("\n");
};

const renderExecutionContextSection = (input: ErrorManifestInput): string => {
  return [
    `- manifest_id: ${escapeMarkdownText(input.id)}`,
    `- job_id: ${escapeMarkdownText(input.jobId ?? "not recorded")}`,
    `- process_id: ${escapeMarkdownText(input.processId ?? "not recorded")}`,
    `- agent_session_id: ${escapeMarkdownText(
      input.agentSessionId ?? "not recorded"
    )}`,
    `- pipeline_stage: ${escapeMarkdownText(input.pipelineStage)}`,
    `- workflow_step: ${escapeMarkdownText(input.workflowStep)}`,
    `- skill_path: ${escapeMarkdownText(input.skillPath ?? "not recorded")}`
  ].join("\n");
};

const renderRuntimeContextSection = (
  runtimeContext: ErrorRuntimeContext
): string => {
  return [
    `- cli_vendor: ${escapeMarkdownText(runtimeContext.cliVendor ?? "unknown")}`,
    `- cli_version: ${escapeMarkdownText(runtimeContext.cliVersion ?? "unknown")}`,
    `- adapter_version: ${escapeMarkdownText(
      runtimeContext.adapterVersion ?? "unknown"
    )}`,
    `- model_id: ${escapeMarkdownText(runtimeContext.modelId ?? "unknown")}`,
    `- config_hash: ${escapeMarkdownText(runtimeContext.configHash ?? "unknown")}`,
    `- skill_version: ${escapeMarkdownText(
      runtimeContext.skillVersion ?? "unknown"
    )}`,
    `- pid: ${escapeMarkdownText(formatNumeric(runtimeContext.pid))}`,
    `- pgid: ${escapeMarkdownText(formatNumeric(runtimeContext.pgid))}`,
    `- cleanup_verdict: ${escapeMarkdownText(
      runtimeContext.cleanupVerdict ?? "unknown"
    )}`,
    `- last_valid_checkpoint_sequence: ${escapeMarkdownText(
      formatNumeric(runtimeContext.lastValidCheckpointSequence)
    )}`,
    `- daemon_instance_id: ${escapeMarkdownText(
      runtimeContext.daemonInstanceId ?? "unknown"
    )}`,
    `- commit_seq: ${escapeMarkdownText(formatNumeric(runtimeContext.commitSeq))}`
  ].join("\n");
};

const renderErrorContextSection = (errorContext: unknown): string => {
  const json = stringifyForMarkdown(errorContext);
  return makeCodeFence(json, "json");
};

const renderLinkedArtifactsSection = (
  linkedArtifacts: readonly LinkedArtifact[]
): string => {
  if (linkedArtifacts.length === 0) {
    return "No linked artifacts recorded.";
  }

  return linkedArtifacts
    .map(
      (artifact) =>
        `- ${escapeMarkdownText(artifact.label)}: ${escapeMarkdownText(
          artifact.path
        )}`
    )
    .join("\n");
};

const renderNextDebuggingActionSection = (
  nextDebuggingAction: string
): string => {
  return escapeMarkdownText(nextDebuggingAction);
};

const renderResolutionSection = (input: ErrorManifestInput): string => {
  return [
    `- resolved: ${input.resolved ? "true" : "false"}`,
    `- resolved_at: ${escapeMarkdownText(input.resolvedAt ?? "unresolved")}`,
    `- resolved_by: ${escapeMarkdownText(input.resolvedBy ?? "unassigned")}`
  ].join("\n");
};

const stringifyForMarkdown = (value: unknown): string => {
  const rendered = JSON.stringify(value ?? {}, null, 2) ?? "{}";
  return rendered
    .replace(/</g, "\\u003c")
    .replace(/>/g, "\\u003e")
    .replace(/&/g, "\\u0026");
};

const makeCodeFence = (body: string, infoString: string): string => {
  const backtickRuns = body.match(/`+/g) ?? [];
  const longestRun = backtickRuns.reduce(
    (max, run) => Math.max(max, run.length),
    3
  );
  const fence = "`".repeat(longestRun + 1);
  return `${fence}${infoString}\n${body}\n${fence}`;
};

const escapeMarkdownText = (
  value: string | number | boolean | null | undefined
): string => {
  const normalized =
    value === null || value === undefined ? "unknown" : String(value);
  return normalized
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\\/g, "\\\\")
    .replace(/([`*{}\[\]()])/g, "\\$1");
};

const normalizeString = (
  value: string | null | undefined
): string | undefined => {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
};

const formatNumeric = (value: number | null | undefined): string => {
  return value === null || value === undefined ? "unknown" : String(value);
};

const isPlainObject = (value: unknown): value is Record<string, unknown> => {
  return typeof value === "object" && value !== null && !Array.isArray(value);
};
