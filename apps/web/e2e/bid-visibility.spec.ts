import { expect, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const WEB_ROOT = dirname(dirname(fileURLToPath(import.meta.url)));

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
