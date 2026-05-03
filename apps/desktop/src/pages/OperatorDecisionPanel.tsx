import { useEffect, useId, useMemo, useState } from 'react';
import type { ReactElement } from 'react';

import { invoke, parseBridgeError } from '../lib/transport';

export type ClarificationPrompt = {
  candidate_id: string;
  candidate_name: string;
  lot_id: string | null;
  reason_code: string;
  prompt_text: string;
  prompt_unit: string | null;
  prompt_type: string;
  default_value: unknown;
  rationale: string | null;
  options?: string[] | null;
};

export type DecisionHistoryEntry = {
  intentId: string;
  idempotencyKey: string;
  disposition: string;
  status: string;
  reviewedBasisHash: string;
  reviewedManifestHash: string;
  reviewedAt: string;
  queuedAt: string;
  supersededByIntentId: string | null;
  decisionNote: string | null;
};

export type ActiveOverrideEntry = {
  overrideId: string;
  productCandidateId: string;
  reasonCode: string;
  payloadJson: string;
  notes: string | null;
  basisHashWhenProvided: string | null;
  providedAt: string;
  providedBy: string;
};

type QueryEnvelope<T> = {
  data: T;
  snapshotSeq: number;
  snapshotAt: string;
};

type Disposition = 'approve' | 'reject' | 'watch';

const RERUN_STAGE_DELAY_MS = 3_000;

const PIPELINE_RERUN_STAGES = [
  { id: 'landed-cost', label: 'landed-cost' },
  { id: 'net-gate', label: 'net-gate' },
  { id: 'manifest', label: 'manifest' },
] as const;

type PipelineRerunStageId = (typeof PIPELINE_RERUN_STAGES)[number]['id'];

type PipelineRerunStatus = {
  state: 'idle' | 'running' | 'success' | 'error';
  message: string | null;
  activeStage: PipelineRerunStageId | null;
  modalOpen: boolean;
};

export type OperatorDecisionPanelProps = {
  processId: string;
  manifestBasisHash: string | null;
  manifestHash: string | null;
  manifestStageRunId: string | null;
  clarificationPromptsJson: string | null;
  /** Latest decision rolled up from the detail view. */
  latestDecision: {
    disposition: 'approve' | 'reject' | 'watch' | null;
    status?: string | null;
    isStale: boolean;
    decidedAt: string | null;
  };
  /** Called when an operator action completes (decision, clarification, rerun)
   * and the parent should refetch the detail view. */
  onChanged: () => void;
  /** Test injection point — bypasses real IPC. */
  loaders?: {
    decisionHistory: (
      processId: string,
    ) => Promise<QueryEnvelope<DecisionHistoryEntry[]>>;
    activeOverrides: (
      processId: string,
    ) => Promise<QueryEnvelope<ActiveOverrideEntry[]>>;
  };
  /** Test injection point — bypasses real IPC for mutations. */
  mutators?: {
    captureDecision?: (args: Record<string, unknown>) => Promise<unknown>;
    captureClarification?: (args: Record<string, unknown>) => Promise<unknown>;
    withdrawClarification?: (args: Record<string, unknown>) => Promise<unknown>;
    withdrawDecision?: (args: Record<string, unknown>) => Promise<unknown>;
    triggerPipelineRerun?: (args: Record<string, unknown>) => Promise<unknown>;
  };
  /** Stage 14 Phase 2 — SSR test seam: pre-populate decision history so the
   * component renders the audit chain without running useEffect. */
  seedHistory?: DecisionHistoryEntry[];
  /** Stage 14 Phase 2 — SSR test seam: pre-populate active overrides. */
  seedActiveOverrides?: ActiveOverrideEntry[];
  /** Stage 14 Phase 2 — SSR test seam: pre-populate rerun progress UI. */
  seedRerunStatus?: PipelineRerunStatus;
};

function parsePrompts(raw: string | null): ClarificationPrompt[] {
  if (!raw) return [];
  try {
    const value = JSON.parse(raw);
    if (Array.isArray(value)) {
      return value as ClarificationPrompt[];
    }
    return [];
  } catch {
    return [];
  }
}

function buildIdempotencyKey(prefix: string): string {
  return `${prefix}-${(globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`)}`;
}

function defaultPayloadFor(prompt: ClarificationPrompt): Record<string, unknown> {
  if (
    prompt.prompt_type === 'number' &&
    typeof prompt.default_value === 'number'
  ) {
    return { qty: prompt.default_value, unit: prompt.prompt_unit ?? 'each' };
  }
  if (prompt.prompt_type === 'object' && prompt.default_value && typeof prompt.default_value === 'object') {
    return { ...(prompt.default_value as Record<string, unknown>) };
  }
  if (prompt.prompt_type === 'choice') {
    return { variant: prompt.options?.[0] ?? '' };
  }
  return {};
}

function promptKey(prompt: ClarificationPrompt): string {
  return `${prompt.candidate_id}::${prompt.reason_code}`;
}

function overridePayload(override: ActiveOverrideEntry): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(override.payloadJson);
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    return null;
  }
  return null;
}

function promptFromActiveOverride(
  override: ActiveOverrideEntry,
): ClarificationPrompt | null {
  if (
    override.reasonCode !== 'quantity_basis_unaligned' &&
    override.reasonCode !== 'quantity_basis_undetermined'
  ) {
    return null;
  }
  const payload = overridePayload(override);
  const qty = Number(payload?.qty);
  if (!Number.isFinite(qty)) {
    return null;
  }
  const unit = typeof payload?.unit === 'string' ? payload.unit : null;
  return {
    candidate_id: override.productCandidateId,
    candidate_name: override.productCandidateId,
    lot_id: null,
    reason_code: override.reasonCode,
    prompt_text: 'Update active quantity override',
    prompt_unit: unit,
    prompt_type: 'number',
    default_value: qty,
    rationale:
      'An active quantity override exists for this candidate; edit it here to supersede the prior value.',
    options: null,
  };
}

function mergePromptsWithActiveOverrides(
  prompts: ClarificationPrompt[],
  activeOverrides: ActiveOverrideEntry[],
): ClarificationPrompt[] {
  const promptKeys = new Set(prompts.map(promptKey));
  const overridePrompts = activeOverrides
    .map(promptFromActiveOverride)
    .filter((prompt): prompt is ClarificationPrompt => {
      return prompt !== null && !promptKeys.has(promptKey(prompt));
    });
  return [...overridePrompts, ...prompts];
}

function seedPayloadsForPrompts(
  prompts: ClarificationPrompt[],
): Record<string, Record<string, unknown>> {
  const seed: Record<string, Record<string, unknown>> = {};
  for (const prompt of prompts) {
    seed[promptKey(prompt)] = defaultPayloadFor(prompt);
  }
  return seed;
}

export function assertNotBridgeErrorResponse(response: unknown): void {
  const bridge = parseBridgeError(response);
  if (bridge) {
    throw bridge;
  }
}

export function OperatorDecisionPanel({
  processId,
  manifestBasisHash,
  manifestHash,
  manifestStageRunId,
  clarificationPromptsJson,
  latestDecision,
  onChanged,
  loaders,
  mutators,
  seedHistory,
  seedActiveOverrides,
  seedRerunStatus,
}: OperatorDecisionPanelProps): ReactElement {
  const reasoningId = useId();
  const prompts = useMemo(
    () => parsePrompts(clarificationPromptsJson),
    [clarificationPromptsJson],
  );

  // ---------- Quick-Decide state ----------
  const [reasoning, setReasoning] = useState('');
  const [decisionStatus, setDecisionStatus] = useState<{
    state: 'idle' | 'saving' | 'success' | 'error';
    message: string | null;
  }>({ state: 'idle', message: null });

  // ---------- Decision history state ----------
  const [history, setHistory] = useState<DecisionHistoryEntry[]>(seedHistory ?? []);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [historyTick, setHistoryTick] = useState(0);

  // ---------- Active overrides state ----------
  const [activeOverrides, setActiveOverrides] = useState<ActiveOverrideEntry[]>(
    seedActiveOverrides ?? [],
  );
  const [overrideError, setOverrideError] = useState<string | null>(null);

  // ---------- Clarification form state ----------
  const displayPrompts = useMemo(
    () => mergePromptsWithActiveOverrides(prompts, activeOverrides),
    [activeOverrides, prompts],
  );
  const [promptPayloads, setPromptPayloads] = useState<
    Record<string, Record<string, unknown>>
  >(() => seedPayloadsForPrompts(displayPrompts));
  const [clarificationStatus, setClarificationStatus] = useState<{
    state: 'idle' | 'saving' | 'success' | 'error';
    message: string | null;
    pendingKey: string | null;
  }>({ state: 'idle', message: null, pendingKey: null });

  // ---------- Pipeline rerun state ----------
  const [rerunStatus, setRerunStatus] = useState<PipelineRerunStatus>(
    seedRerunStatus ?? {
      state: 'idle',
      message: null,
      activeStage: null,
      modalOpen: false,
    },
  );

  // Reload history + overrides
  useEffect(() => {
    let cancelled = false;
    const decisionLoader =
      loaders?.decisionHistory ??
      ((id: string) =>
        invoke('get_decision_history', { processId: id }) as Promise<
          QueryEnvelope<DecisionHistoryEntry[]>
        >);
    const overrideLoader =
      loaders?.activeOverrides ??
      ((id: string) =>
        invoke('get_active_overrides', { processId: id }) as Promise<
          QueryEnvelope<ActiveOverrideEntry[]>
        >);

    decisionLoader(processId)
      .then((envelope) => {
        if (cancelled) return;
        setHistory(envelope.data);
        setHistoryError(null);
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        const bridge = parseBridgeError(error);
        setHistoryError(bridge?.message ?? 'Failed to load decision history');
      });

    overrideLoader(processId)
      .then((envelope) => {
        if (cancelled) return;
        setActiveOverrides(envelope.data);
        setOverrideError(null);
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        const bridge = parseBridgeError(error);
        setOverrideError(bridge?.message ?? 'Failed to load active overrides');
      });

    return () => {
      cancelled = true;
    };
  }, [processId, historyTick, loaders]);

  useEffect(() => {
    setPromptPayloads((current) => {
      let changed = false;
      const next = { ...current };
      for (const prompt of displayPrompts) {
        const key = promptKey(prompt);
        if (!next[key]) {
          next[key] = defaultPayloadFor(prompt);
          changed = true;
        }
      }
      return changed ? next : current;
    });
  }, [displayPrompts]);

  async function handleCaptureDecision(disposition: Disposition): Promise<void> {
    if (!manifestBasisHash || !manifestHash || !manifestStageRunId) {
      setDecisionStatus({
        state: 'error',
        message: 'Cannot capture decision: manifest metadata is incomplete.',
      });
      return;
    }
    setDecisionStatus({ state: 'saving', message: null });
    try {
      const args = {
        processId,
        disposition,
        reviewedBasisHash: manifestBasisHash,
        reviewedManifestHash: manifestHash,
        reviewedAtStageRunId: manifestStageRunId,
        decisionNote: reasoning.trim() || null,
        idempotencyKey: buildIdempotencyKey(`decision-${disposition}`),
      };
      const fn = mutators?.captureDecision ?? ((a) => invoke('capture_decision', a));
      assertNotBridgeErrorResponse(await fn(args));
      setDecisionStatus({
        state: 'success',
        message: `Decision captured: ${disposition}`,
      });
      setReasoning('');
      setHistoryTick((tick) => tick + 1);
      onChanged();
    } catch (error: unknown) {
      const bridge = parseBridgeError(error);
      setDecisionStatus({
        state: 'error',
        message: bridge?.message ?? 'Failed to capture decision',
      });
    }
  }

  async function handleSubmitClarification(prompt: ClarificationPrompt): Promise<void> {
    const key = promptKey(prompt);
    setClarificationStatus({ state: 'saving', message: null, pendingKey: key });
    const payload = promptPayloads[key] ?? defaultPayloadFor(prompt);
    try {
      const args = {
        processId,
        candidateId: prompt.candidate_id,
        reasonCode: prompt.reason_code,
        payload,
        idempotencyKey: buildIdempotencyKey('clarification'),
      };
      const fn =
        mutators?.captureClarification ?? ((a) => invoke('capture_clarification', a));
      assertNotBridgeErrorResponse(await fn(args));
      setClarificationStatus({
        state: 'success',
        message: `Clarification captured for ${prompt.candidate_name}`,
        pendingKey: null,
      });
      setHistoryTick((tick) => tick + 1);
      onChanged();
    } catch (error: unknown) {
      const bridge = parseBridgeError(error);
      setClarificationStatus({
        state: 'error',
        message: bridge?.message ?? 'Failed to capture clarification',
        pendingKey: null,
      });
    }
  }

  async function handleWithdrawClarification(prompt: ClarificationPrompt): Promise<void> {
    setClarificationStatus({
      state: 'saving',
      message: null,
      pendingKey: `${prompt.candidate_id}::${prompt.reason_code}`,
    });
    try {
      const args = {
        processId,
        candidateId: prompt.candidate_id,
        reasonCode: prompt.reason_code,
        idempotencyKey: buildIdempotencyKey('withdraw-clarification'),
      };
      const fn =
        mutators?.withdrawClarification ?? ((a) => invoke('withdraw_clarification', a));
      assertNotBridgeErrorResponse(await fn(args));
      setClarificationStatus({
        state: 'success',
        message: `Clarification withdrawn for ${prompt.candidate_name}`,
        pendingKey: null,
      });
      setHistoryTick((tick) => tick + 1);
      onChanged();
    } catch (error: unknown) {
      const bridge = parseBridgeError(error);
      setClarificationStatus({
        state: 'error',
        message: bridge?.message ?? 'Failed to withdraw clarification',
        pendingKey: null,
      });
    }
  }

  function handlePipelineRerun(): void {
    setRerunStatus({
      state: 'running',
      message: 'Running landed-cost…',
      activeStage: 'landed-cost',
      modalOpen: true,
    });
    try {
      const args = { processId };
      const fn =
        mutators?.triggerPipelineRerun ??
        ((a) => invoke('trigger_pipeline_rerun', a));
      const commandResult = fn(args)
        .then((response) => {
          assertNotBridgeErrorResponse(response);
          return { ok: true as const };
        })
        .catch((error: unknown) => ({ ok: false as const, error }));

      PIPELINE_RERUN_STAGES.forEach((stage, index) => {
        globalThis.setTimeout(() => {
          setRerunStatus((current) => {
            if (current.state !== 'running') {
              return current;
            }
            return {
              state: 'running',
              message: `Running ${stage.label}…`,
              activeStage: stage.id,
              modalOpen: true,
            };
          });
        }, index * RERUN_STAGE_DELAY_MS);
      });

      globalThis.setTimeout(() => {
        void commandResult.then((result) => {
          if (!result.ok) {
            const bridge = parseBridgeError(result.error);
            setRerunStatus({
              state: 'error',
              message: bridge?.message ?? 'Failed to trigger pipeline rerun',
              activeStage: null,
              modalOpen: false,
            });
            return;
          }
          setRerunStatus({
            state: 'success',
            message: 'Pipeline rerun queued. Manifest will refresh shortly.',
            activeStage: 'manifest',
            modalOpen: false,
          });
          onChanged();
        });
      }, PIPELINE_RERUN_STAGES.length * RERUN_STAGE_DELAY_MS);
    } catch (error: unknown) {
      const bridge = parseBridgeError(error);
      setRerunStatus({
        state: 'error',
        message: bridge?.message ?? 'Failed to trigger pipeline rerun',
        activeStage: null,
        modalOpen: false,
      });
    }
  }

  async function handleWithdrawDecision(intent: DecisionHistoryEntry): Promise<void> {
    try {
      const args = { idempotencyKey: intent.idempotencyKey };
      const fn = mutators?.withdrawDecision ?? ((a) => invoke('withdraw_decision', a));
      assertNotBridgeErrorResponse(await fn(args));
      setHistoryTick((tick) => tick + 1);
      onChanged();
    } catch (error: unknown) {
      const bridge = parseBridgeError(error);
      setHistoryError(bridge?.message ?? 'Failed to withdraw decision');
    }
  }

  // Stale banner — surfaces when the latest queued intent is marked stale,
  // i.e. the manifest_basis_hash drifted after the decision was reviewed.
  const staleBanner = latestDecision.isStale ? (
    <div
      role="alert"
      data-testid="stale-banner"
      style={{
        background: '#fff3cd',
        color: '#664d03',
        padding: '8px 12px',
        borderRadius: '4px',
        border: '1px solid #ffe69c',
        marginBottom: '12px',
      }}
    >
      This decision was made against an earlier manifest version. Re-confirm or withdraw.
    </div>
  ) : null;

  return (
    <article
      className="detail-card"
      data-testid="operator-decision-panel"
      style={{ display: 'grid', gap: '16px' }}
    >
      <h3>Operator decision (Stage 14)</h3>
      {staleBanner}

      <section data-testid="quick-decide">
        <h4>Quick-Decide</h4>
        <p style={{ color: '#666', fontSize: '0.9em' }}>
          Capture the final disposition. Reasoning is optional but recommended.
        </p>
        <label htmlFor={reasoningId}>
          Reasoning
          <textarea
            id={reasoningId}
            data-testid="reasoning-textarea"
            value={reasoning}
            onChange={(e) => setReasoning(e.target.value)}
            placeholder="Why are you capturing this disposition?"
            rows={3}
            style={{ width: '100%', display: 'block' }}
          />
        </label>
        <div style={{ display: 'flex', gap: '8px', marginTop: '8px' }}>
          <button
            type="button"
            data-testid="quick-decide-approve"
            onClick={() => void handleCaptureDecision('approve')}
            disabled={decisionStatus.state === 'saving'}
          >
            Approve
          </button>
          <button
            type="button"
            data-testid="quick-decide-reject"
            onClick={() => void handleCaptureDecision('reject')}
            disabled={decisionStatus.state === 'saving'}
          >
            Reject
          </button>
          <button
            type="button"
            data-testid="quick-decide-watch"
            onClick={() => void handleCaptureDecision('watch')}
            disabled={decisionStatus.state === 'saving'}
          >
            Watch
          </button>
        </div>
        {decisionStatus.message ? (
          <p
            role={decisionStatus.state === 'error' ? 'alert' : 'status'}
            data-testid="decision-status"
          >
            {decisionStatus.message}
          </p>
        ) : null}
      </section>

      <section data-testid="clarification-form">
        <h4>Clarifications ({displayPrompts.length})</h4>
        {displayPrompts.length === 0 ? (
          <p className="empty-state">No clarification prompts on this manifest.</p>
        ) : null}
        {displayPrompts.map((prompt) => {
          const key = promptKey(prompt);
          const payload = promptPayloads[key] ?? defaultPayloadFor(prompt);
          return (
            <fieldset
              key={key}
              data-testid="clarification-prompt"
              data-reason-code={prompt.reason_code}
              data-candidate-id={prompt.candidate_id}
              style={{ border: '1px solid #ddd', padding: '8px', marginBottom: '8px' }}
            >
              <legend>
                {prompt.candidate_name} — {prompt.reason_code}
              </legend>
              <p style={{ fontSize: '0.9em' }}>{prompt.prompt_text}</p>
              {prompt.rationale ? (
                <p style={{ fontStyle: 'italic', color: '#666', fontSize: '0.85em' }}>
                  {prompt.rationale}
                </p>
              ) : null}
              {prompt.prompt_type === 'number' ? (
                <label>
                  Quantity
                  <input
                    type="number"
                    data-testid={`prompt-input-${prompt.candidate_id}`}
                    value={String(payload.qty ?? '')}
                    onChange={(e) =>
                      setPromptPayloads((prev) => ({
                        ...prev,
                        [key]: {
                          ...payload,
                          qty: Number(e.target.value),
                          unit: prompt.prompt_unit ?? 'each',
                        },
                      }))
                    }
                  />
                  {prompt.prompt_unit ? <span> {prompt.prompt_unit}</span> : null}
                </label>
              ) : null}
              {prompt.prompt_type === 'choice' && prompt.options ? (
                <label>
                  Choice
                  <select
                    data-testid={`prompt-select-${prompt.candidate_id}`}
                    value={String(payload.variant ?? '')}
                    onChange={(e) =>
                      setPromptPayloads((prev) => ({
                        ...prev,
                        [key]: { ...payload, variant: e.target.value },
                      }))
                    }
                  >
                    {prompt.options.map((option) => (
                      <option key={option} value={option}>
                        {option}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}
              {prompt.prompt_type === 'object' ? (
                <label>
                  Payload (JSON)
                  <textarea
                    data-testid={`prompt-json-${prompt.candidate_id}`}
                    value={JSON.stringify(payload, null, 2)}
                    onChange={(e) => {
                      try {
                        const next = JSON.parse(e.target.value);
                        setPromptPayloads((prev) => ({ ...prev, [key]: next }));
                      } catch {
                        // ignore parse errors while editing
                      }
                    }}
                    rows={4}
                    style={{ width: '100%', display: 'block', fontFamily: 'monospace' }}
                  />
                </label>
              ) : null}
              <div style={{ display: 'flex', gap: '8px', marginTop: '8px' }}>
                <button
                  type="button"
                  data-testid={`submit-clarification-${prompt.candidate_id}`}
                  onClick={() => void handleSubmitClarification(prompt)}
                  disabled={
                    clarificationStatus.state === 'saving' &&
                    clarificationStatus.pendingKey === key
                  }
                >
                  Submit clarification
                </button>
                <button
                  type="button"
                  data-testid={`withdraw-clarification-${prompt.candidate_id}`}
                  onClick={() => void handleWithdrawClarification(prompt)}
                >
                  Withdraw
                </button>
              </div>
            </fieldset>
          );
        })}
        {clarificationStatus.message ? (
          <p
            role={clarificationStatus.state === 'error' ? 'alert' : 'status'}
            data-testid="clarification-status"
          >
            {clarificationStatus.message}
          </p>
        ) : null}
      </section>

      <section data-testid="active-overrides">
        <h4>Active overrides ({activeOverrides.length})</h4>
        {overrideError ? (
          <p role="alert">{overrideError}</p>
        ) : activeOverrides.length === 0 ? (
          <p className="empty-state">No active overrides.</p>
        ) : (
          <ul className="stack-list">
            {activeOverrides.map((override) => (
              <li key={override.overrideId} data-testid="active-override">
                <strong>{override.reasonCode}</strong> for {override.productCandidateId}
                <pre style={{ fontSize: '0.85em' }}>{override.payloadJson}</pre>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section data-testid="pipeline-rerun">
        <h4>Pipeline rerun</h4>
        <button
          type="button"
          data-testid="trigger-pipeline-rerun"
          onClick={() => void handlePipelineRerun()}
          disabled={rerunStatus.state === 'running'}
        >
          {rerunStatus.state === 'running' ? 'Re-running…' : 'Re-run pipeline'}
        </button>
        {rerunStatus.message ? (
          <p
            role={rerunStatus.state === 'error' ? 'alert' : 'status'}
            data-testid="rerun-status"
          >
            {rerunStatus.message}
          </p>
        ) : null}
        {rerunStatus.state === 'running' && rerunStatus.modalOpen ? (
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="pipeline-rerun-progress-title"
            data-testid="rerun-progress-modal"
            data-active-stage={rerunStatus.activeStage ?? undefined}
            style={{
              border: '1px solid #b6c8db',
              borderRadius: '6px',
              padding: '12px',
              marginTop: '12px',
              background: '#f7fbff',
            }}
          >
            <h5 id="pipeline-rerun-progress-title">Re-run progress</h5>
            <ol style={{ display: 'grid', gap: '6px', paddingLeft: '20px' }}>
              {PIPELINE_RERUN_STAGES.map((stage) => (
                <li
                  key={stage.id}
                  data-testid="rerun-progress-stage"
                  data-stage={stage.id}
                  data-active={rerunStatus.activeStage === stage.id}
                  style={{
                    fontWeight: rerunStatus.activeStage === stage.id ? 700 : 400,
                  }}
                >
                  {stage.label}
                </li>
              ))}
            </ol>
          </div>
        ) : null}
      </section>

      <section data-testid="decision-history">
        <h4>Decision history ({history.length})</h4>
        {historyError ? <p role="alert">{historyError}</p> : null}
        {history.length === 0 ? (
          <p className="empty-state">No prior decisions for this process.</p>
        ) : (
          <ul className="stack-list">
            {history.map((entry) => (
              <li
                key={entry.intentId}
                data-testid="decision-history-entry"
                data-status={entry.status}
                data-disposition={entry.disposition}
              >
                <strong>{entry.disposition.toUpperCase()}</strong>
                <span
                  data-testid="decision-history-status"
                  style={{
                    background: statusColor(entry.status),
                    color: '#fff',
                    padding: '2px 6px',
                    borderRadius: '4px',
                    marginLeft: '6px',
                    fontSize: '0.8em',
                  }}
                >
                  {entry.status}
                </span>
                <p style={{ fontSize: '0.85em', color: '#666' }}>
                  {entry.queuedAt} · key {entry.idempotencyKey}
                </p>
                {entry.decisionNote ? (
                  <p style={{ fontSize: '0.9em' }}>{entry.decisionNote}</p>
                ) : null}
                {entry.status === 'queued' || entry.status === 'stale' ? (
                  <button
                    type="button"
                    data-testid="withdraw-decision-button"
                    onClick={() => void handleWithdrawDecision(entry)}
                  >
                    Withdraw decision
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </section>
    </article>
  );
}

function statusColor(status: string): string {
  switch (status) {
    case 'queued':
      return '#1f8a4f';
    case 'superseded':
      return '#888';
    case 'stale':
      return '#e0a200';
    case 'withdrawn':
      return '#c0392b';
    default:
      return '#5d6d7e';
  }
}

export default OperatorDecisionPanel;
