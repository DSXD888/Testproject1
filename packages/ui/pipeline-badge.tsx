import type { ReactElement } from 'react';

export type PipelineState =
  | 'pending'
  | 'qualifying'
  | 'enriching'
  | 'qualified'
  | 'review'
  | 'rejected'
  | 'error';

export type ValidationState = 'unverified' | 'validated' | 'flagged';

export interface PipelineBadgeProps {
  state: PipelineState;
  validationState?: ValidationState;
  size?: 'sm' | 'md' | 'lg';
}

interface BadgeMeta {
  icon: string;
  label: string;
  tone: 'muted' | 'info' | 'good' | 'warn' | 'bad';
}

const PIPELINE_META: Record<PipelineState, BadgeMeta> = {
  pending: { icon: 'PD', label: 'Pending', tone: 'muted' },
  qualifying: { icon: 'QF', label: 'Qualifying', tone: 'info' },
  enriching: { icon: 'EN', label: 'Enriching', tone: 'info' },
  qualified: { icon: 'OK', label: 'Qualified', tone: 'good' },
  review: { icon: 'RV', label: 'Review', tone: 'warn' },
  rejected: { icon: 'NO', label: 'Rejected', tone: 'bad' },
  error: { icon: 'ER', label: 'Error', tone: 'bad' },
};

const VALIDATION_META: Record<Exclude<ValidationState, 'unverified'>, BadgeMeta> = {
  validated: { icon: 'VD', label: 'Validated', tone: 'good' },
  flagged: { icon: 'FG', label: 'Flagged', tone: 'warn' },
};

function renderBadge(meta: BadgeMeta, size: PipelineBadgeProps['size']): ReactElement {
  return (
    <span
      className="pipeline-badge"
      data-size={size ?? 'md'}
      data-tone={meta.tone}
      title={`${meta.label} state`}
    >
      <span aria-hidden="true" className="pipeline-badge__icon">
        {meta.icon}
      </span>
      <span className="pipeline-badge__label">{meta.label}</span>
    </span>
  );
}

export function PipelineBadge({
  state,
  validationState = 'unverified',
  size = 'md',
}: PipelineBadgeProps): ReactElement {
  const stateBadge = renderBadge(PIPELINE_META[state], size);
  const validationBadge =
    validationState === 'unverified'
      ? null
      : renderBadge(VALIDATION_META[validationState], size);

  return (
    <span className="pipeline-badge-stack" data-state={state}>
      {stateBadge}
      {validationBadge}
    </span>
  );
}
