import type { ReactElement } from 'react';

export interface FreshnessBadgeVM {
  label: string;
  tone: 'good' | 'warn' | 'bad' | 'muted';
  tooltip: string;
}

export interface FreshnessBadgeProps {
  vm: FreshnessBadgeVM;
}

const TONE_ICON: Record<FreshnessBadgeVM['tone'], string> = {
  good: 'OK',
  warn: 'WR',
  bad: 'ST',
  muted: 'IN',
};

export function FreshnessBadge({ vm }: FreshnessBadgeProps): ReactElement {
  return (
    <span
      className="freshness-badge"
      data-tone={vm.tone}
      title={vm.tooltip}
      role="status"
    >
      <span aria-hidden="true" className="freshness-badge__icon">
        {TONE_ICON[vm.tone]}
      </span>
      <span className="freshness-badge__label">{vm.label}</span>
    </span>
  );
}
