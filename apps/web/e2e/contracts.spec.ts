import { expect, test } from '../../e2e-playwright-compat';
import { readdir, readFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  MUTATION_RPC_NAMES,
  READABLE_PROJECTION_TABLES,
  REMOTE_OWNED_TABLES,
} from '@project/contracts-schema';
import { buildSyncFreshnessVm } from '@project/ui';
import {
  PRIVILEGED_TOKEN_NAME_PATTERN,
  viewerCredentialNames,
} from './fixtures/supabase-auth';
import {
  FRESHNESS_NOW,
  freshnessProjectionSeeds,
} from './fixtures/projection-seed';
import {
  contractDetailPageTopics,
  contractsPageTopics,
  errorsTopic,
  isPortalUpdate,
  processUpdatesTopic,
  statusPageTopics,
  statusTopic,
  syncTopic,
  type PortalUpdate,
} from '../src/lib/broadcast';

const WEB_ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const REPO_ROOT = dirname(dirname(WEB_ROOT));

async function listSourceFiles(dir: string): Promise<string[]> {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const path = join(dir, entry.name);

      if (entry.isDirectory()) {
        return listSourceFiles(path);
      }

      return ['.ts', '.tsx'].some((extension) => entry.name.endsWith(extension))
        ? [path]
        : [];
    }),
  );

  return files.flat();
}

test('L4-T-contracts_spec-001 redirects the root route to contracts', async ({
  page,
}) => {
  await page.goto('/');
  await expect(page).toHaveURL(/\/contracts$/);
  await expect(page.getByTestId('global-sync-status')).toBeVisible();
  await expect(page.getByTestId('contracts-list-page')).toBeVisible();
});

test('L4-T-contracts_spec-002 binds the root layout to viewer_sync_status only through the web boundary', async () => {
  const layoutSource = await readFile(join(WEB_ROOT, 'src/app/layout.tsx'), 'utf8');
  const pageSource = await readFile(join(WEB_ROOT, 'src/app/page.tsx'), 'utf8');

  expect(pageSource).toContain("replace('/contracts')");
  expect(layoutSource).toContain('getViewerSyncStatus');
  expect(layoutSource).toContain('FreshnessBadge');
  expect(layoutSource).toContain('viewer_sync_status');
  expect(layoutSource).not.toMatch(/@supabase\/supabase-js|tauri|sqlite|ipc|invoke/i);
});

test('L4-T-contracts_spec-003 keeps the list page read-only', async () => {
  const pageSource = await readFile(join(WEB_ROOT, 'src/app/contracts/page.tsx'), 'utf8');

  expect(pageSource).toContain('ContractCardFrame');
  expect(pageSource).toContain('listContractSummariesPage');
  expect(pageSource).toContain('subscribeToContractsPage');
  expect(pageSource).not.toMatch(
    /approve\s*\(|reject\s*\(|watch\s*\(|agent control|daemon trigger|config mutation/i,
  );
});

test('L4-T-contracts_spec-004 uses keyset pagination for the contract list', async () => {
  const boundarySource = await readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8');

  expect(boundarySource).toContain('listContractSummariesPage');
  expect(boundarySource).toContain("order('latest_updated_at', { ascending: false })");
  expect(boundarySource).toContain("order('id', { ascending: false })");
  expect(boundarySource).toContain('latest_updated_at.lt');
  expect(boundarySource).toContain('latest_updated_at.eq');
  expect(boundarySource).toContain('id.lt');
  expect(boundarySource).not.toMatch(/\.range\(|\.offset\(/);
});

test('L4-T-contracts_spec-008 keeps live list updates non-disruptive', async () => {
  const pageSource = await readFile(join(WEB_ROOT, 'src/app/contracts/page.tsx'), 'utf8');

  expect(pageSource).toContain('contracts-update-pill');
  expect(pageSource).toContain('setPendingUpdateCount');
  expect(pageSource).toContain('setCursor(null)');
  expect(pageSource).not.toMatch(/sort\(|reverse\(/);
});

test('L4-T-contracts_spec-009 detail minor updates refresh affected sections only', async () => {
  const pageSource = await readFile(
    join(WEB_ROOT, 'src/app/contracts/[id]/page.tsx'),
    'utf8',
  );

  expect(pageSource).toContain('subscribeToContractDetailPage');
  expect(pageSource).toContain('refreshMinorSections');
  expect(pageSource).toContain('mergeMinorSections');
  expect(pageSource).toContain("update.changed.includes('quotes')");
  expect(pageSource).toContain("update.changed.includes('errors')");
  expect(pageSource).toContain('getContractDetail');
  expect(pageSource).not.toMatch(/setState\(\{\s*status:\s*'loading'/);
});

test('L4-T-contracts_spec-010 detail major updates wait behind a refresh chip', async () => {
  const pageSource = await readFile(
    join(WEB_ROOT, 'src/app/contracts/[id]/page.tsx'),
    'utf8',
  );

  expect(pageSource).toContain('contract-detail-refresh-chip');
  expect(pageSource).toContain('pendingMajorUpdateCount');
  expect(pageSource).toContain('isMajorProcessUpdate');
  expect(pageSource).toContain("'canonical_process'");
  expect(pageSource).toContain("'landed_cost'");
  expect(pageSource).toContain("'manifest'");
  expect(pageSource).toContain('replaceVisibleSnapshot');
});

test('L4-T-contracts_spec-011 detail renders projection summaries through shared UI', async () => {
  const pageSource = await readFile(
    join(WEB_ROOT, 'src/app/contracts/[id]/page.tsx'),
    'utf8',
  );
  const boundarySource = await readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8');

  expect(pageSource).toContain('PipelineBadge');
  expect(pageSource).toContain('ManifestViewer');
  expect(pageSource).toContain('ErrorSummaryList');
  expect(pageSource).toContain('Decision summary');
  expect(pageSource).toContain('Candidates');
  expect(pageSource).toContain('Quotes');
  expect(pageSource).toContain('Margins');
  expect(pageSource).toContain('Goods-led labour and permits');
  expect(pageSource).toContain('installation_labour_coverage_bps');
  expect(pageSource).toContain('installation_labour_value_coverage_bps');
  expect(pageSource).toContain('permit_traffic_mgmt_coverage_bps');
  expect(pageSource).toContain('permit_traffic_mgmt_value_coverage_bps');
  expect(pageSource).toContain('installation_labour_cost_gbp_pence');
  expect(pageSource).toContain('permit_traffic_mgmt_cost_gbp_pence');
  expect(pageSource).toContain('permit_responsibility');
  expect(boundarySource).toContain("from('product_candidates')");
  expect(boundarySource).toContain("from('supplier_quotes')");
  expect(boundarySource).toContain("from('candidate_pricing_assessments')");
  expect(boundarySource).toContain("from('landed_cost_estimates')");
  expect(boundarySource).toContain("from('manifests')");
  expect(boundarySource).toContain("from('operator_decision_summaries')");
  expect(boundarySource).toContain("from('error_manifest_summaries')");
  expect(pageSource).not.toMatch(
    /approve\s*\(|reject\s*\(|watch\s*\(|Rerun enrichment|Run .* ingestion|daemon trigger|config mutation/i,
  );
});

test('L4-T-contracts_spec-011 detail route renders read-only fallback snapshot', async ({
  page,
}) => {
  await page.goto('/contracts/process-7');

  await expect(page.getByTestId('contract-detail-page')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Decision summary' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Goods-led labour and permits' })).toBeVisible();
  await expect(page.getByText('Labour coverage')).toBeVisible();
  await expect(page.getByText('Labour value coverage')).toBeVisible();
  await expect(page.getByText('Installation labour base')).toBeVisible();
  await expect(page.getByText('Permit/TM coverage')).toBeVisible();
  await expect(page.getByText('Permit/TM value coverage')).toBeVisible();
  await expect(page.getByText('Permit/TM base')).toBeVisible();
  await expect(page.getByText('Permit responsibility')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Candidates' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Quotes' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Manifest' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Errors' })).toBeVisible();
  await expect(
    page.getByText(/approve|reject|watch|Rerun enrichment|Run .* ingestion|daemon trigger/i),
  ).toHaveCount(0);
});

test('L4-T-contracts_spec-012 status page renders summary projections only', async () => {
  const pageSource = await readFile(join(WEB_ROOT, 'src/app/status/page.tsx'), 'utf8');
  const boundarySource = await readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8');

  expect(pageSource).toContain('getStatusPageSnapshot');
  expect(pageSource).toContain('FreshnessBadge');
  expect(pageSource).toContain('PipelineBadge');
  expect(pageSource).toContain('ErrorSummaryList');
  expect(pageSource).toContain('viewer_sync_status');
  expect(pageSource).toContain('workflow_job_summaries');
  expect(pageSource).toContain('error_manifest_summaries');
  expect(pageSource).toContain('projection_generation');
  expect(boundarySource).toContain('getStatusPageSnapshot');
  expect(boundarySource).toContain('getViewerSyncStatus');
  expect(boundarySource).toContain('listWorkflowJobSummaries');
  expect(boundarySource).toContain("from('viewer_sync_status')");
  expect(boundarySource).toContain("from('workflow_job_summaries')");
  expect(boundarySource).toContain("from('error_manifest_summaries')");
  expect(pageSource).not.toMatch(
    /approve\s*\(|reject\s*\(|watch\s*\(|Restart daemon|Open logs|Open artifacts|Quit UI|stop_background_daemon_and_quit|daemon trigger|config mutation|invoke\(/i,
  );
});

test('L4-T-contracts_spec-019 status generation changes replace stale snapshots', async () => {
  const pageSource = await readFile(join(WEB_ROOT, 'src/app/status/page.tsx'), 'utf8');

  expect(pageSource).toContain('subscribeToStatusPage');
  expect(pageSource).toContain('projectionGeneration');
  expect(pageSource).toContain('activeProjectionGeneration');
  expect(pageSource).toContain('replaceStatusSnapshot');
  expect(pageSource).toContain('pendingProjectionGeneration');
  expect(pageSource).toContain('status-generation-refresh-chip');
  expect(pageSource).not.toContain('mergeStatusSnapshot');
});

test('L4-T-contracts_spec-012 status route renders read-only fallback snapshot', async ({
  page,
}) => {
  await page.goto('/status');

  await expect(page.getByTestId('status-page')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Pipeline status' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Sync health' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Workflow jobs' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Errors' })).toBeVisible();
  await expect(
    page.getByText(/Restart daemon|Open logs|Open artifacts|Quit UI|daemon trigger/i),
  ).toHaveCount(0);
});

test('L4-T-contracts_spec-017 verifies desktop-seeded projections stay visible and read-only in the web portal', async ({
  page,
}) => {
  const [listSource, detailSource, statusSource, boundarySource] = await Promise.all([
    readFile(join(WEB_ROOT, 'src/app/contracts/page.tsx'), 'utf8'),
    readFile(join(WEB_ROOT, 'src/app/contracts/[id]/page.tsx'), 'utf8'),
    readFile(join(WEB_ROOT, 'src/app/status/page.tsx'), 'utf8'),
    readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8'),
  ]);
  const viewerSource = [listSource, detailSource, statusSource, boundarySource].join('\n');

  expect(listSource).toContain('listContractSummariesPage');
  expect(listSource).toContain('href={`/contracts/${row.id}`}');
  expect(detailSource).toContain('getContractDetail(processId, orgId)');
  expect(detailSource).toContain('Decision summary');
  expect(detailSource).toContain('Goods-led labour and permits');
  expect(detailSource).toContain('installation_labour_coverage_bps');
  expect(detailSource).toContain('permit_traffic_mgmt_coverage_bps');
  expect(detailSource).toContain('installation_labour_cost_gbp_pence');
  expect(detailSource).toContain('permit_traffic_mgmt_cost_gbp_pence');
  expect(detailSource).toContain('disposition');
  expect(statusSource).toContain('getStatusPageSnapshot(orgId)');
  expect(statusSource).toContain('viewer_sync_status');
  expect(boundarySource).toContain("from('procurement_processes')");
  expect(boundarySource).toContain("from('operator_decision_summaries')");
  expect(boundarySource).toContain("from('viewer_sync_status')");
  expect(boundarySource).toContain("from('workflow_job_summaries')");
  expect(boundarySource).toContain("from('error_manifest_summaries')");
  expect(boundarySource).toContain('.eq(\'process_id\', processId)');
  expect(boundarySource).toContain('.eq(\'org_id\', orgId)');
  expect(boundarySource).not.toMatch(/\.insert\(|\.update\(|\.delete\(|\.rpc\(/);
  expect(viewerSource).not.toMatch(
    /approve\s*\(|reject\s*\(|watch\s*\(|Rerun enrichment|Run .* ingestion|daemon trigger|config mutation|invoke\(/i,
  );

  await page.goto('/contracts/process-7');
  await expect(page.getByTestId('global-sync-status')).toBeVisible();
  await expect(page.getByTestId('contract-detail-page')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Decision summary' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Candidates' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Manifest' })).toBeVisible();
  await expect(page.getByText(/approve|reject|Rerun enrichment|Run .* ingestion/i)).toHaveCount(0);

  await page.goto('/status');
  await expect(page.getByTestId('status-page')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Sync health' })).toBeVisible();
  await expect(page.getByTestId('sync-projection-generation')).toBeVisible();
  await expect(page.getByText(/Restart daemon|Open logs|Open artifacts|daemon trigger/i)).toHaveCount(0);
});

test('L4-T-contracts_spec-018 degrades safely when sync or Broadcast is unavailable', async ({
  page,
}) => {
  const [contractsSource, detailSource, statusSource, broadcastSource, boundarySource] =
    await Promise.all([
      readFile(join(WEB_ROOT, 'src/app/contracts/page.tsx'), 'utf8'),
      readFile(join(WEB_ROOT, 'src/app/contracts/[id]/page.tsx'), 'utf8'),
      readFile(join(WEB_ROOT, 'src/app/status/page.tsx'), 'utf8'),
      readFile(join(WEB_ROOT, 'src/lib/broadcast.ts'), 'utf8'),
      readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8'),
    ]);
  const degradedVm = buildSyncFreshnessVm({
    syncHealth: 'degraded',
    lastSuccessAt: '2026-03-28T18:27:30Z',
    now: FRESHNESS_NOW,
  });
  const offlineVm = buildSyncFreshnessVm({
    syncHealth: 'offline',
    lastSuccessAt: '2026-03-28T18:29:15Z',
    now: FRESHNESS_NOW,
  });
  const degradedSource = [
    contractsSource,
    detailSource,
    statusSource,
    broadcastSource,
    boundarySource,
  ].join('\n');

  expect(degradedVm).toMatchObject({ tone: 'warn' });
  expect(offlineVm).toMatchObject({ tone: 'bad' });
  expect(statusSource).toContain('fallbackSyncVm');
  expect(statusSource).toContain('viewer_sync_status is unavailable');
  expect(statusSource).toContain('setPendingProjectionGeneration(update.projectionGeneration)');
  expect(statusSource).toContain('snapshot: current.snapshot');
  expect(statusSource).not.toContain('mergeStatusSnapshot');
  expect(detailSource).toContain('snapshot: current.snapshot');
  expect(contractsSource).toContain('isMissingViewerConfig(error)');
  expect(broadcastSource).toContain("event: 'sync.updated'");
  expect(broadcastSource).toContain('isAllowedString(value.syncHealth, SYNC_HEALTH_VALUES)');
  expect(boundarySource).not.toMatch(/\.insert\(|\.update\(|\.delete\(|\.rpc\(/);
  expect(degradedSource).not.toMatch(
    /service.?role|admin.?key|operator.?token|daemon.?token|Restart daemon|Open logs|Open artifacts|Quit UI|invoke\(/i,
  );

  await page.goto('/status');
  await expect(page.getByTestId('global-sync-status')).toBeVisible();
  await expect(page.getByTestId('status-page')).toBeVisible();
  await expect(page.getByText('Sync offline -- showing last known data').first()).toBeVisible();
  await expect(page.getByText(/Restart daemon|Open logs|Open artifacts|daemon trigger/i)).toHaveCount(0);
});

test('L4-T-contracts_spec-020 keeps Supabase behind the web boundary', async () => {
  const srcFiles = await listSourceFiles(join(WEB_ROOT, 'src'));
  const directSupabaseImports: string[] = [];

  for (const filePath of srcFiles) {
    const source = await readFile(filePath, 'utf8');

    if (source.includes('@supabase/supabase-js')) {
      directSupabaseImports.push(relative(WEB_ROOT, filePath));
    }
  }

  expect(directSupabaseImports).toEqual(['src/lib/supabase.ts']);
  expect(await readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8')).toContain(
    '@project/contracts-schema',
  );
});

test('L4-T-contracts_spec-013-015 exposes viewer-only schema and env names', async () => {
  const boundarySource = await readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8');
  const sourceFiles = await listSourceFiles(join(WEB_ROOT, 'src'));
  const sourceText = (
    await Promise.all(sourceFiles.map((filePath) => readFile(filePath, 'utf8')))
  ).join('\n');

  expect(boundarySource).toContain('NEXT_PUBLIC_SUPABASE_URL');
  expect(boundarySource).toContain('NEXT_PUBLIC_SUPABASE_ANON_KEY');
  expect(boundarySource).not.toMatch(/service.?role|admin.?key|operator.?token|daemon.?token/i);
  expect(sourceText).not.toMatch(/127\.0\.0\.1:\d+|localhost:\d+|tauri|sqlite|ipc/i);
  expect(READABLE_PROJECTION_TABLES).toContain('viewer_sync_status');
  expect(READABLE_PROJECTION_TABLES).toContain('operator_decision_summaries');
  expect(REMOTE_OWNED_TABLES).toEqual([
    'shared_company_profile',
    'shared_source_configs',
    'operator_decision_heads',
  ]);
  expect(MUTATION_RPC_NAMES).toEqual(['apply_push_batch', 'update_operator_decision']);
  expect(relative(REPO_ROOT, join(WEB_ROOT, 'src/lib/supabase.ts'))).toBe(
    'apps/web/src/lib/supabase.ts',
  );
});

test('L4-T-contracts_spec-005-007 maps viewer_sync_status to freshness states', () => {
  const states = freshnessProjectionSeeds.map((seed) =>
    buildSyncFreshnessVm({
      syncHealth: seed.syncHealth,
      lastSuccessAt: seed.lastPushSuccessAt,
      now: FRESHNESS_NOW,
    }),
  );

  expect(states).toEqual([
    {
      label: 'Data current',
      tone: 'good',
      tooltip: 'Last successful sync: 2026-03-28T18:29:15Z',
    },
    {
      label: 'Updates delayed -- data may be behind desktop',
      tone: 'warn',
      tooltip: 'Last successful sync: 2026-03-28T18:27:30Z',
    },
    {
      label: 'Sync offline -- showing last known data',
      tone: 'bad',
      tooltip: 'Last successful sync: 2026-03-28T18:29:15Z',
    },
  ]);
});

test('L4-T-contracts_spec-016 keeps UI and web bundle boundaries read-only', async () => {
  const uiFiles = await listSourceFiles(join(REPO_ROOT, 'packages/ui'));
  const webFiles = await listSourceFiles(join(WEB_ROOT, 'src'));
  const uiSource = (
    await Promise.all(uiFiles.map((filePath) => readFile(filePath, 'utf8')))
  ).join('\n');
  const webSource = (
    await Promise.all(webFiles.map((filePath) => readFile(filePath, 'utf8')))
  ).join('\n');

  expect(uiSource).not.toMatch(/@tauri-apps\/api|apps\/desktop|crates\/daemon|supabase|transport/i);
  expect(viewerCredentialNames()).toEqual([
    'NEXT_PUBLIC_SUPABASE_URL',
    'NEXT_PUBLIC_SUPABASE_ANON_KEY',
  ]);
  expect(viewerCredentialNames().join('\n')).not.toMatch(PRIVILEGED_TOKEN_NAME_PATTERN);
  expect(webSource).not.toMatch(PRIVILEGED_TOKEN_NAME_PATTERN);
});

test('L4-T-contracts_spec-008-010-019 defines page-scoped Broadcast topics', () => {
  expect(contractsPageTopics('org-1')).toEqual([
    'org:org-1:contracts',
    'org:org-1:sync',
  ]);
  expect(contractDetailPageTopics('org-1', 'process-7')).toEqual([
    'process:process-7:updates',
    'org:org-1:sync',
  ]);
  expect(statusPageTopics('org-1')).toEqual([
    'org:org-1:status',
    'org:org-1:errors',
    'org:org-1:sync',
  ]);
  expect(processUpdatesTopic('process-7')).toBe('process:process-7:updates');
  expect(statusTopic('org-1')).toBe('org:org-1:status');
  expect(errorsTopic('org-1')).toBe('org:org-1:errors');
  expect(syncTopic('org-1')).toBe('org:org-1:sync');
});

test('L4-T-contracts_spec-008-010-019 accepts only typed invalidation payloads', () => {
  const contractsUpdate = {
    event: 'contracts.changed',
    processIds: ['process-1', 'process-2'],
    changed: ['assessment', 'decision', 'manifest', 'error'],
    maxSeq: 42,
  } satisfies PortalUpdate;
  const minorProcessUpdate = {
    event: 'process.updated',
    processId: 'process-1',
    changed: ['quotes', 'landed_cost', 'manifest', 'jobs', 'errors', 'canonical_process'],
    major: false,
    maxSeq: 43,
  } satisfies PortalUpdate;
  const syncUpdate = {
    event: 'sync.updated',
    syncHealth: 'degraded',
    lastAckedSeq: 43,
    lastSuccessAt: '2026-03-28T18:27:30Z',
    projectionGeneration: 2,
  } satisfies PortalUpdate;

  expect(isPortalUpdate(contractsUpdate)).toBe(true);
  expect(isPortalUpdate(minorProcessUpdate)).toBe(true);
  expect(isPortalUpdate({ ...minorProcessUpdate, major: true })).toBe(true);
  expect(isPortalUpdate(syncUpdate)).toBe(true);
  expect(
    isPortalUpdate({
      event: 'contracts.changed',
      processIds: ['process-1'],
      changed: ['quotes'],
      maxSeq: 44,
    }),
  ).toBe(false);
  expect(
    isPortalUpdate({
      event: 'sync.updated',
      syncHealth: 'healthy',
      lastSuccessAt: '2026-03-28T18:29:15Z',
      projectionGeneration: 3,
    }),
  ).toBe(false);
});

test('L4-T-contracts_spec-008-010-019 keeps Broadcast behind the Supabase boundary', async () => {
  const broadcastSource = await readFile(join(WEB_ROOT, 'src/lib/broadcast.ts'), 'utf8');

  expect(broadcastSource).toContain("from './supabase'");
  expect(broadcastSource).not.toContain('@supabase/supabase-js');
  expect(broadcastSource).toContain(".on('broadcast'");
  expect(broadcastSource).toContain("{ event: '*' }");
  expect(broadcastSource).not.toMatch(
    /ProcurementProcessRow|ViewerSyncStatusRow|WorkflowJobSummaryRow|ErrorManifestSummaryRow/,
  );
  expect(broadcastSource).not.toContain('.from(');
});
