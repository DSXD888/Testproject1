import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import {
  AutonomyStatusPanel,
  type CronStatusSnapshot,
} from './DaemonStatus';

function cronStatusFixture(): CronStatusSnapshot {
  return {
    definitions: [
      {
        definitionKey: 'fts::procurement-ingestion/fts',
        sourceId: 'fts',
        skillId: 'procurement-ingestion/fts',
        skillVersion: '1.0.0',
        cronExpression: '15 4 * * *',
        enabled: true,
        lastRunAt: '2026-03-28T04:15:00Z',
        nextRunAt: '2026-03-29T04:15:00Z',
      },
    ],
    schedulerRunning: true,
    lastTickAt: '2026-03-28T20:00:00Z',
    nextRunAt: '2026-03-29T04:15:00Z',
    workerLanes: [
      {
        lane: 'pty_browser',
        running: true,
        claimableKinds: ['sync_source', 'source_health', 'auth_refresh'],
      },
      {
        lane: 'coverage_deterministic',
        running: true,
        claimableKinds: [
          'commercial_sweep',
          'validation_cross_model_audit',
          'validation_calibration_readiness_sweep',
        ],
      },
    ],
    awaitingAccess: {
      activeCount: 2,
      expiredCount: 1,
    },
  };
}

describe('DaemonStatusPage', () => {
  it('desktop_status_001 renders autonomy status read-only', () => {
    const html = renderToStaticMarkup(
      <AutonomyStatusPanel key="autonomy-status" cronStatus={cronStatusFixture()} />,
    );

    expect(html).toContain('Autonomy status');
    expect(html).toContain('Scheduler');
    expect(html).toContain('running');
    expect(html).toContain('Last scheduler tick');
    expect(html).toContain('2026-03-28T20:00:00Z');
    expect(html).toContain('Next scheduled run');
    expect(html).toContain('2026-03-29T04:15:00Z');
    expect(html).toContain('Awaiting ProContract access');
    expect(html).toContain('Expired awaiting access');
    expect(html).toContain('pty_browser');
    expect(html).toContain('coverage_deterministic');
    expect(html).toContain('sync_source, source_health, auth_refresh');
    expect(html).not.toContain('<button');
    expect(html).not.toContain('<input');
    expect(html).not.toContain('<select');
    expect(html).not.toContain('<textarea');
  });
});
