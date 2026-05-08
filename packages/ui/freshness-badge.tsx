import type { ReactElement } from 'react';

export interface FreshnessBadgeVM {
  label: string;
  tone: 'good' | 'warn' | 'bad' | 'muted';
  tooltip: string;
}

export type SyncHealth = 'healthy' | 'degraded' | 'offline';

export interface SyncFreshnessInput {
  syncHealth: SyncHealth;
  lastSuccessAt: string;
  now?: Date | number | string;
}

export type FreshnessBadgeProps =
  | {
      vm: FreshnessBadgeVM;
      syncHealth?: never;
      lastSuccessAt?: never;
      now?: never;
    }
  | ({
      vm?: never;
    } & SyncFreshnessInput);

const TONE_ICON: Record<FreshnessBadgeVM['tone'], string> = {
  good: 'OK',
  warn: 'WR',
  bad: 'ST',
  muted: 'IN',
};

const HEALTHY_MAX_AGE_MS = 90 * 1000;
const DEGRADED_MAX_AGE_MS = 5 * 60 * 1000;

function toTime(value: Date | number | string): number {
  return value instanceof Date ? value.getTime() : new Date(value).getTime();
}

export function buildSyncFreshnessVm(input: SyncFreshnessInput): FreshnessBadgeVM {
  const lastSuccessAtMs = toTime(input.lastSuccessAt);
  const nowMs = input.now === undefined ? Date.now() : toTime(input.now);
  const ageMs = nowMs - lastSuccessAtMs;
  const isOffline =
    input.syncHealth === 'offline' ||
    !Number.isFinite(lastSuccessAtMs) ||
    ageMs > DEGRADED_MAX_AGE_MS;

  if (isOffline) {
    return {
      label: 'Sync offline -- showing last known data',
      tone: 'bad',
      tooltip: `Last successful sync: ${input.lastSuccessAt}`,
    };
  }

  if (input.syncHealth === 'degraded' || ageMs >= HEALTHY_MAX_AGE_MS) {
    return {
      label: 'Updates delayed -- data may be behind desktop',
      tone: 'warn',
      tooltip: `Last successful sync: ${input.lastSuccessAt}`,
    };
  }

  return {
    label: 'Data current',
    tone: 'good',
    tooltip: `Last successful sync: ${input.lastSuccessAt}`,
  };
}

export function FreshnessBadge(props: FreshnessBadgeProps): ReactElement {
  const vm =
    props.vm ?? buildSyncFreshnessVm({
      syncHealth: props.syncHealth,
      lastSuccessAt: props.lastSuccessAt,
      now: props.now,
    });

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
