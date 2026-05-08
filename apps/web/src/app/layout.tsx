import type { Metadata } from 'next';
import type { CSSProperties, ReactNode } from 'react';
import {
  FreshnessBadge,
  type FreshnessBadgeVM,
} from '@project/ui';
import type { ViewerSyncStatusRow } from '@project/contracts-schema';
import { getViewerSyncStatus } from '../lib/supabase';

export const dynamic = 'force-dynamic';

export const metadata: Metadata = {
  title: 'Procurement Intelligence',
  description: 'Read-only shared visibility portal',
};

const DEFAULT_VIEWER_ORG_ID = 'default';

const shellStyle: CSSProperties = {
  minHeight: '100vh',
  background: '#f7f8fa',
  color: '#15171a',
  fontFamily:
    'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
};

const headerStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: 24,
  padding: '16px 24px',
  borderBottom: '1px solid #d7dce2',
  background: '#ffffff',
};

const navStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 16,
  fontSize: 14,
  fontWeight: 600,
};

const statusStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 16,
  flexWrap: 'wrap',
  justifyContent: 'flex-end',
  fontSize: 12,
};

const metricListStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 12,
  margin: 0,
};

const metricStyle: CSSProperties = {
  display: 'flex',
  gap: 4,
  margin: 0,
  color: '#59616b',
};

const metricValueStyle: CSSProperties = {
  margin: 0,
  color: '#15171a',
  fontWeight: 700,
};

const mainStyle: CSSProperties = {
  minHeight: 'calc(100vh - 65px)',
};

const fallbackSyncVm: FreshnessBadgeVM = {
  label: 'Sync offline -- showing last known data',
  tone: 'bad',
  tooltip: 'viewer_sync_status is unavailable',
};

function getViewerOrgId(): string {
  return process.env.NEXT_PUBLIC_SUPABASE_ORG_ID?.trim() || DEFAULT_VIEWER_ORG_ID;
}

async function loadViewerSyncStatus(): Promise<ViewerSyncStatusRow | null> {
  try {
    return await getViewerSyncStatus(getViewerOrgId());
  } catch {
    return null;
  }
}

function renderFreshnessBadge(status: ViewerSyncStatusRow | null): ReactNode {
  if (!status?.last_push_success_at) {
    return <FreshnessBadge vm={fallbackSyncVm} />;
  }

  return (
    <FreshnessBadge
      syncHealth={status.sync_health}
      lastSuccessAt={status.last_push_success_at}
    />
  );
}

function renderMetric(value: number | string | null | undefined): string {
  return value === null || value === undefined ? 'n/a' : String(value);
}

export default async function RootLayout({ children }: { children: ReactNode }) {
  const syncStatus = await loadViewerSyncStatus();

  return (
    <html lang="en">
      <body style={{ margin: 0 }}>
        <div style={shellStyle}>
          <header style={headerStyle}>
            <nav aria-label="Primary" style={navStyle}>
              <a href="/contracts">Contracts</a>
              <a href="/status">Status</a>
            </nav>
            <section
              aria-label="Sync freshness"
              data-projection="viewer_sync_status"
              data-testid="global-sync-status"
              style={statusStyle}
            >
              {renderFreshnessBadge(syncStatus)}
              <dl aria-label="Sync metrics" style={metricListStyle}>
                <div style={metricStyle}>
                  <dt>Pending</dt>
                  <dd
                    data-testid="sync-pending-outbox-count"
                    style={metricValueStyle}
                  >
                    {renderMetric(syncStatus?.pending_outbox_count)}
                  </dd>
                </div>
                <div style={metricStyle}>
                  <dt>Schema</dt>
                  <dd data-testid="sync-schema-epoch" style={metricValueStyle}>
                    {renderMetric(syncStatus?.schema_epoch)}
                  </dd>
                </div>
                <div style={metricStyle}>
                  <dt>Generation</dt>
                  <dd
                    data-testid="sync-projection-generation"
                    style={metricValueStyle}
                  >
                    {renderMetric(syncStatus?.projection_generation)}
                  </dd>
                </div>
              </dl>
            </section>
          </header>
          <main style={mainStyle}>{children}</main>
        </div>
      </body>
    </html>
  );
}
