import type { SyncHealth } from '@project/ui';

export interface ViewerSyncStatusSeed {
  orgId: string;
  syncHealth: SyncHealth;
  lastPushSuccessAt: string;
  projectionGeneration: number;
}

export const FRESHNESS_NOW = '2026-03-28T18:30:00Z';

export const freshnessProjectionSeeds = [
  {
    orgId: 'org-healthy',
    syncHealth: 'healthy',
    lastPushSuccessAt: '2026-03-28T18:29:15Z',
    projectionGeneration: 12,
  },
  {
    orgId: 'org-degraded',
    syncHealth: 'healthy',
    lastPushSuccessAt: '2026-03-28T18:27:30Z',
    projectionGeneration: 12,
  },
  {
    orgId: 'org-offline',
    syncHealth: 'offline',
    lastPushSuccessAt: '2026-03-28T18:29:15Z',
    projectionGeneration: 12,
  },
] as const satisfies readonly ViewerSyncStatusSeed[];

