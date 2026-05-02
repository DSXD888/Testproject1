import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import {
  OperatorDecisionPanel,
  type ClarificationPrompt,
  type DecisionHistoryEntry,
} from './OperatorDecisionPanel';

const PROMPTS_NUMBER: ClarificationPrompt[] = [
  {
    candidate_id: 'cand-1',
    candidate_name: 'DGU 1.2m²',
    lot_id: 'lot-1',
    reason_code: 'quantity_basis_unaligned',
    prompt_text: 'How many units?',
    prompt_unit: 'm²',
    prompt_type: 'number',
    default_value: 250,
    rationale: 'Quantity not inferred',
    options: null,
  },
];

const PROMPTS_CHOICE: ClarificationPrompt[] = [
  {
    candidate_id: 'cand-2',
    candidate_name: 'Roofing',
    lot_id: null,
    reason_code: 'no_pricing_signal',
    prompt_text: 'Pick a variant',
    prompt_unit: null,
    prompt_type: 'choice',
    default_value: null,
    rationale: null,
    options: ['manual_price', 'no_priced_supplier'],
  },
];

const PROMPTS_OBJECT: ClarificationPrompt[] = [
  {
    candidate_id: 'cand-3',
    candidate_name: 'RFQ candidate',
    lot_id: null,
    reason_code: 'awaiting_rfq_quote',
    prompt_text: 'Provide RFQ payload',
    prompt_unit: null,
    prompt_type: 'object',
    default_value: { unit_price_gbp_pence: 100, lead_time_days: 30, moq: 1, currency: 'GBP' },
    rationale: null,
    options: null,
  },
];

function panel(props: {
  prompts?: ClarificationPrompt[];
  history?: DecisionHistoryEntry[];
  isStale?: boolean;
}) {
  return renderToStaticMarkup(
    <OperatorDecisionPanel
      processId="proc-1"
      manifestBasisHash="hash-1"
      manifestHash="manifest-1"
      manifestStageRunId="run-1"
      clarificationPromptsJson={
        props.prompts ? JSON.stringify(props.prompts) : null
      }
      latestDecision={{
        disposition: null,
        isStale: props.isStale ?? false,
        decidedAt: null,
      }}
      onChanged={() => {}}
      seedHistory={props.history}
    />,
  );
}

describe('FT.4 — Quick-Decide panel surfaces approve/reject/watch with reasoning input', () => {
  it('renders the reasoning textarea and three disposition buttons', () => {
    const html = panel({});
    expect(html).toContain('data-testid="reasoning-textarea"');
    expect(html).toContain('data-testid="quick-decide-approve"');
    expect(html).toContain('data-testid="quick-decide-reject"');
    expect(html).toContain('data-testid="quick-decide-watch"');
  });
});

describe('FT.5 — Clarification form maps prompt_type to input control', () => {
  it('renders a number input for prompt_type=number', () => {
    const html = panel({ prompts: PROMPTS_NUMBER });
    expect(html).toContain('data-testid="prompt-input-cand-1"');
    expect(html).toContain('type="number"');
  });

  it('renders a select for prompt_type=choice with all options', () => {
    const html = panel({ prompts: PROMPTS_CHOICE });
    expect(html).toContain('data-testid="prompt-select-cand-2"');
    expect(html).toContain('manual_price');
    expect(html).toContain('no_priced_supplier');
  });

  it('renders a JSON textarea for prompt_type=object', () => {
    const html = panel({ prompts: PROMPTS_OBJECT });
    expect(html).toContain('data-testid="prompt-json-cand-3"');
    expect(html).toContain('unit_price_gbp_pence');
  });
});

describe('FT.6 — Each prompt has Submit + Withdraw buttons wired to its candidate', () => {
  it('emits a submit-clarification button per prompt', () => {
    const html = panel({ prompts: PROMPTS_NUMBER });
    expect(html).toContain('data-testid="submit-clarification-cand-1"');
    expect(html).toContain('data-testid="withdraw-clarification-cand-1"');
  });
});

describe('FT.7 — Stale banner appears when latest decision isStale', () => {
  it('renders the stale banner when isStale=true', () => {
    const html = panel({ isStale: true });
    expect(html).toContain('data-testid="stale-banner"');
    expect(html).toContain('Re-confirm or withdraw');
  });

  it('does not render the stale banner when isStale=false', () => {
    const html = panel({ isStale: false });
    expect(html).not.toContain('data-testid="stale-banner"');
  });
});

describe('FT.8 — Decision history audit chain renders entries with status', () => {
  it('renders one row per decision history entry with status badges', () => {
    const history: DecisionHistoryEntry[] = [
      {
        intentId: 'intent-1',
        idempotencyKey: 'key-1',
        disposition: 'approve',
        status: 'queued',
        reviewedBasisHash: 'h1',
        reviewedManifestHash: 'm1',
        reviewedAt: '2026-04-01T12:00:00Z',
        queuedAt: '2026-04-01T12:00:00Z',
        supersededByIntentId: null,
        decisionNote: 'Looks good',
      },
      {
        intentId: 'intent-2',
        idempotencyKey: 'key-2',
        disposition: 'watch',
        status: 'stale',
        reviewedBasisHash: 'h0',
        reviewedManifestHash: 'm0',
        reviewedAt: '2026-04-01T11:00:00Z',
        queuedAt: '2026-04-01T11:00:00Z',
        supersededByIntentId: 'intent-1',
        decisionNote: null,
      },
      {
        intentId: 'intent-3',
        idempotencyKey: 'key-3',
        disposition: 'watch',
        status: 'superseded',
        reviewedBasisHash: 'h2',
        reviewedManifestHash: 'm2',
        reviewedAt: '2026-04-01T10:30:00Z',
        queuedAt: '2026-04-01T10:30:00Z',
        supersededByIntentId: 'intent-1',
        decisionNote: null,
      },
      {
        intentId: 'intent-4',
        idempotencyKey: 'key-4',
        disposition: 'reject',
        status: 'withdrawn',
        reviewedBasisHash: 'h3',
        reviewedManifestHash: 'm3',
        reviewedAt: '2026-04-01T10:00:00Z',
        queuedAt: '2026-04-01T10:00:00Z',
        supersededByIntentId: null,
        decisionNote: null,
      },
    ];
    const html = panel({ history });
    expect(html).toContain('data-status="queued"');
    expect(html).toContain('data-status="stale"');
    expect(html).toContain('data-status="superseded"');
    expect(html).toContain('data-status="withdrawn"');
    // Withdraw button appears for queued and stale intents.
    expect(html.match(/data-testid="withdraw-decision-button"/g)?.length).toBe(2);
    expect(html).toContain('Looks good'); // decision_note rendered
  });

  it('bug_stale_decision_withdraw_regression_renders_withdraw_button', () => {
    const history: DecisionHistoryEntry[] = [
      {
        intentId: 'intent-stale',
        idempotencyKey: 'key-stale',
        disposition: 'approve',
        status: 'stale',
        reviewedBasisHash: 'h1',
        reviewedManifestHash: 'm1',
        reviewedAt: '2026-04-01T12:00:00Z',
        queuedAt: '2026-04-01T12:00:00Z',
        supersededByIntentId: null,
        decisionNote: null,
      },
    ];
    const html = panel({ history });
    expect(html).toContain('data-status="stale"');
    expect(html).toContain('data-testid="withdraw-decision-button"');
  });
});
