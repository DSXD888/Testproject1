import type { FreshnessBadgeVM, SyncFreshnessInput } from '../freshness-badge';

export const freshnessBadgeVmFixture: FreshnessBadgeVM = {
  label: '2 min ago',
  tone: 'warn',
  tooltip: 'Last local refresh: 2026-03-28T18:21:00Z',
};

export const healthyFreshnessFixture: SyncFreshnessInput = {
  syncHealth: 'healthy',
  lastSuccessAt: '2026-03-28T18:21:00Z',
  now: '2026-03-28T18:21:45Z',
};

export const degradedFreshnessFixture: SyncFreshnessInput = {
  syncHealth: 'healthy',
  lastSuccessAt: '2026-03-28T18:21:00Z',
  now: '2026-03-28T18:23:00Z',
};

export const offlineFreshnessFixture: SyncFreshnessInput = {
  syncHealth: 'offline',
  lastSuccessAt: '2026-03-28T18:21:00Z',
  now: '2026-03-28T18:21:45Z',
};
