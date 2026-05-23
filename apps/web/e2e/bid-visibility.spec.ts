import { expect, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  BID_WORKFLOW_SENSITIVE_COLUMNS,
  BID_WORKFLOW_SUMMARY_COLUMNS,
  WEB_RLS_CATEGORIES,
  WEB_RLS_ROLES,
  WEB_RLS_SENSITIVE_TOKENS,
  buildRoleScopedSelectAttempts,
  buildWebRlsGrid,
  issueRoleScopedSelectAttempts,
  type RoleScopedAttemptResult,
  type RoleScopedSelectAttempt,
} from './_rls_grid';

const WEB_ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const REPO_ROOT = dirname(dirname(WEB_ROOT));

function functionBlock(source: string, functionName: string): string {
  const start = source.indexOf(`export async function ${functionName}`);
  expect(start).toBeGreaterThanOrEqual(0);

  const nextFunction = source.indexOf('\nasync function ', start + 1);
  return nextFunction === -1 ? source.slice(start) : source.slice(start, nextFunction);
}

test('L6-T-web_ui-001: viewer sees bid-state badge + timestamp', async ({
  page,
}) => {
  await page.goto('/contracts/process-7');

  await expect(page.getByTestId('contract-detail-page')).toBeVisible();
  await expect(page.getByTestId('bid-state-section')).toBeVisible();
  await expect(page.getByTestId('bid-state-badge')).toContainText(/not started/i);
  await expect(page.getByTestId('bid-state-updated-at')).toHaveAttribute(
    'dateTime',
    /^\d{4}-\d{2}-\d{2}T/,
  );
  await expect(
    page.getByText(/approve|reject|submit bid|formal submission|daemon trigger/i),
  ).toHaveCount(0);
});

test('L6-T-web_ui-001 keeps bid workflow projection read-only and non-sensitive', async () => {
  const boundarySource = await readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8');
  const detailSource = await readFile(
    join(WEB_ROOT, 'src/app/contracts/[id]/page.tsx'),
    'utf8',
  );
  const loaderSource = functionBlock(boundarySource, 'loadBidWorkflowSummary');

  expect(detailSource).toContain('BidWorkflowStatusBadge');
  expect(detailSource).toContain('bid-state-section');
  expect(detailSource).toContain('bid-state-badge');
  expect(detailSource).toContain('bid-state-updated-at');
  expect(detailSource).toContain('bid-error-manifest-link');
  expect(loaderSource).toContain("from(\n    'bid_workflow_states'");
  expect(boundarySource).toContain(
    "'process_id,bid_state,review_required,bid_job_id,updated_at'",
  );
  expect(loaderSource).not.toMatch(
    /bid_scope_json|company_profile|manifest_body|run_manifest|portal_artifact_path|screenshot_path/i,
  );
  expect(detailSource).not.toMatch(
    /bid_scope_json|company_profile|manifest_body|run_manifest|portal_artifact_path|screenshot_path/i,
  );
  expect(detailSource).not.toMatch(
    /approve\s*\(|reject\s*\(|submit bid|formal submission|daemon trigger|invoke\(/i,
  );
});

test('L6-T-web_ui-002/L6-T-sync_propagation-004: role-scoped RLS grid denies sensitive bid projection reads', async () => {
  const [boundarySource, detailSource, rlsSource] = await Promise.all([
    readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8'),
    readFile(join(WEB_ROOT, 'src/app/contracts/[id]/page.tsx'), 'utf8'),
    readFile(join(REPO_ROOT, 'infra/supabase/migrations/0004_rls_policies.sql'), 'utf8'),
  ]);
  const grid = buildWebRlsGrid();
  const loaderSource = functionBlock(boundarySource, 'loadBidWorkflowSummary');
  const summaryColumnGrant = `GRANT SELECT (${BID_WORKFLOW_SUMMARY_COLUMNS.join(', ')}) ON bid_workflow_states TO anon, authenticated`;

  expect(grid).toHaveLength(WEB_RLS_ROLES.length * WEB_RLS_CATEGORIES.length);
  for (const role of WEB_RLS_ROLES) {
    for (const category of WEB_RLS_CATEGORIES) {
      expect(grid).toContainEqual(
        expect.objectContaining({
          role,
          category,
        }),
      );
    }
  }

  expect(boundarySource).toContain(BID_WORKFLOW_SUMMARY_COLUMNS.join(','));
  expect(rlsSource).toContain('REVOKE SELECT ON bid_workflow_states FROM anon, authenticated');
  expect(rlsSource).toContain(summaryColumnGrant);
  expect(rlsSource).toContain("'shared_company_profile'");
  expect(rlsSource).toContain("table_name || '_admin_daemon_select'");
  expect(rlsSource).toContain(
    "l4_request_role() = ''daemon'' OR l4_request_role() = ''admin''",
  );
  expect(rlsSource).not.toContain(
    'GRANT INSERT, UPDATE, DELETE ON bid_workflow_states',
  );
  for (const column of BID_WORKFLOW_SENSITIVE_COLUMNS) {
    expect(boundarySource).not.toContain(column);
    expect(summaryColumnGrant).not.toContain(column);
  }
  for (const source of [boundarySource, detailSource]) {
    for (const token of WEB_RLS_SENSITIVE_TOKENS) {
      expect(source).not.toContain(token);
    }
  }
  expect(boundarySource).not.toContain("from('shared_company_profile')");
  expect(loaderSource).not.toContain(".select('*')");
});

test('L6-T-sync_propagation-004 issues role-scoped PostgREST attempts for every RLS cell', async () => {
  const attempts = buildRoleScopedSelectAttempts('process-7');
  const seen: RoleScopedSelectAttempt[] = [];
  const results = await issueRoleScopedSelectAttempts(
    'process-7',
    async (attempt): Promise<RoleScopedAttemptResult> => {
      seen.push(attempt);

      return {
        attempt,
        status: attempt.access === 'allow_summary' ? 200 : 403,
        body:
          attempt.access === 'allow_summary'
            ? JSON.stringify({ bid_state: 'draft_saved', updated_at: '2026-05-23T12:00:00Z' })
            : JSON.stringify({ code: '42501', message: 'permission denied for web projection' }),
      };
    },
  );

  expect(seen).toHaveLength(WEB_RLS_ROLES.length * WEB_RLS_CATEGORIES.length);
  expect(results).toHaveLength(seen.length);
  for (const result of results) {
    expect(result.attempt.requestRoleHeader).toBe(result.attempt.role);
    expect(result.attempt.path).toContain(`/rest/v1/${result.attempt.table}`);
    if (result.attempt.access === 'allow_summary') {
      expect(result.status).toBe(200);
      for (const column of BID_WORKFLOW_SUMMARY_COLUMNS) {
        expect(result.attempt.columns).toContain(column);
      }
    } else {
      expect(result.status).toBe(403);
      for (const token of WEB_RLS_SENSITIVE_TOKENS) {
        expect(result.body).not.toContain(token);
      }
    }
  }
});

test('L6-T-web_ui-002: viewer DOM and browser network payloads do not expose bid scope, company profile, or artifact paths', async ({
  page,
}) => {
  const networkBodies: string[] = [];

  page.on('response', (response) => {
    const url = response.url();
    if (!/supabase|postgrest|bid_workflow_states/i.test(url)) {
      return;
    }

    void response
      .text()
      .then((body) => {
        networkBodies.push(body);
      })
      .catch(() => undefined);
  });

  await page.goto('/contracts/process-7');
  await expect(page.getByTestId('contract-detail-page')).toBeVisible();

  const html = await page.content();
  for (const token of WEB_RLS_SENSITIVE_TOKENS) {
    expect(html).not.toContain(token);
  }
  for (const body of networkBodies) {
    for (const token of WEB_RLS_SENSITIVE_TOKENS) {
      expect(body).not.toContain(token);
    }
  }
});

test('L6-T-web_ui-003: viewer has no bid mutation controls and web bundle carries no mutation path', async ({
  page,
}) => {
  const [detailSource, boundarySource] = await Promise.all([
    readFile(join(WEB_ROOT, 'src/app/contracts/[id]/page.tsx'), 'utf8'),
    readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8'),
  ]);
  const mutationAttempts = buildRoleScopedSelectAttempts('process-7').filter(
    (attempt) => attempt.category === 'bid_mutation_controls_and_rpcs',
  );

  expect(mutationAttempts).toHaveLength(WEB_RLS_ROLES.length);
  for (const attempt of mutationAttempts) {
    expect(attempt.method).toBe('PATCH');
    expect(attempt.access).toBe('deny_mutation');
  }
  expect([detailSource, boundarySource].join('\n')).not.toMatch(
    /\.insert\(|\.update\(|\.delete\(|\.rpc\(|approve\s*\(|reject\s*\(|submit bid|formal submission|daemon trigger|invoke\(/i,
  );

  await page.goto('/contracts/process-7');
  await expect(page.getByTestId('contract-detail-page')).toBeVisible();
  await expect(page.getByTestId('start-bid-button')).toHaveCount(0);
  await expect(
    page.getByText(/approve|reject|submit bid|formal submission|daemon trigger/i),
  ).toHaveCount(0);
});

test('L6-T-web_ui-004: bid_failed summary is limited to redacted error code and actionable category', async () => {
  const detailSource = await readFile(
    join(WEB_ROOT, 'src/app/contracts/[id]/page.tsx'),
    'utf8',
  );
  const statusSource = await readFile(join(WEB_ROOT, 'src/app/status/page.tsx'), 'utf8');

  expect(detailSource).toContain('bid-error-manifest-link');
  expect(detailSource).toContain("row.error_code ??");
  expect(detailSource).toContain('Redacted error summary available in shared visibility.');
  expect(statusSource).toContain('ErrorSummaryList');
  expect([detailSource, statusSource].join('\n')).not.toMatch(
    /error_context|contextWithSentinel|bid_scope_json|company_profile|portal_artifact_path|forensic_pty_bytes_path|TestCompanyName-Sentinel-L6-2026/i,
  );
});
