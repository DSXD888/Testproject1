import { expect, test } from '@playwright/test';

test('PW-ERR-001 lists unresolved manifests with stable severity', async ({ page }) => {
  await page.goto('/errors?errorsFixture=default');

  await expect(page.getByRole('heading', { name: 'Errors', exact: true })).toBeVisible();
  const errorList = page.locator('.error-summary-list');
  await expect(errorList).toContainText('AGENT_DAEMON_CRASH_DURING_SESSION');
  await expect(errorList).toContainText('Critical');
  await expect(errorList).toContainText('Process');
  await expect(errorList).toContainText('session-live-1');
  await expect(errorList.locator('> li')).toHaveCount(2);
});

test('PW-ERR-002 renders an explicit empty state for an empty unresolved queue', async ({
  page,
}) => {
  await page.goto('/errors?errorsFixture=empty');

  await expect(page.getByRole('heading', { name: 'No unresolved errors' })).toBeVisible();
  await expect(page.getByText('The operator queue is currently clear.')).toBeVisible();
  await expect(page.getByRole('listitem')).toHaveCount(0);
});

test('PW-ERR-003 preserves the severity matrix without synthesizing info rows', async ({
  page,
}) => {
  await page.goto('/errors?errorsFixture=severityMatrix');

  const errorList = page.locator('.error-summary-list');
  await expect(errorList).toContainText('AGENT_DAEMON_CRASH_DURING_SESSION');
  await expect(errorList).toContainText('SOURCE_CHANGED');
  await expect(errorList).toContainText('PIPELINE_QUALITY_GATE_FAILED');
  await expect(errorList).toContainText('Critical');
  await expect(errorList).toContainText('Warning');
  await expect(page.getByText('Info', { exact: true })).toHaveCount(0);
});

test('PW-ERR-004 keeps drill-down content sanitized', async ({ page }) => {
  await page.goto('/errors?errorsFixture=hostile');

  await page.getByRole('button', { name: 'View details' }).click();
  await expect(page.getByLabel('Error details')).toContainText(
    '<svg onload=payload-run>alert(1)</svg>',
  );
  await expect(page.locator('pre[aria-label="Raw error context"]')).toContainText(
    '"agentSessionId": "session-hostile-1"',
  );
  await expect(page.locator('img')).toHaveCount(0);
});

test('PW-ERR-005 resolve removes the row only after the refetch fence clears', async ({
  page,
}) => {
  let reads = 0;

  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as {
      cmd: string;
    };

    if (body.cmd === 'list_errors') {
      reads += 1;
      await route.fulfill({
          json:
            reads <= 2
              ? {
                  data: {
                    items: [
                    {
                      errorManifestId: 'err-live-1',
                      processId: 'proc-001',
                      pipelineStage: 'Enrich',
                      errorType: 'AGENT_DAEMON_CRASH_DURING_SESSION',
                      errorMessage: 'Daemon crash interrupted the supplier enrichment session.',
                      cleanupVerdict: 'leaked',
                      severity: 'critical',
                      createdAt: '2026-03-28T09:15:00Z',
                      resolved: false,
                    },
                  ],
                  nextCursor: null,
                  hasMore: false,
                },
                snapshotSeq: reads <= 2 ? 200 : 201,
                snapshotAt: '2026-03-28T21:45:00Z',
              }
            : {
                data: {
                  items: [],
                  nextCursor: null,
                  hasMore: false,
                },
                snapshotSeq: 201,
                snapshotAt: '2026-03-28T21:45:02Z',
              },
      });
      return;
    }

    if (body.cmd === 'resolve_error') {
      await route.fulfill({
        json: {
          accepted: true,
          visibleAfterSeq: 201,
        },
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/errors');
  await page.getByRole('button', { name: 'Resolve error' }).click();
  await expect(page.getByText('Error resolution committed.')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'No unresolved errors' })).toBeVisible();
});

test('PW-ERR-006 rerun creates a new job while preserving the original error row', async ({
  page,
}) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as {
      cmd: string;
    };

    if (body.cmd === 'list_errors') {
      await route.fulfill({
        json: {
          data: {
            items: [
              {
                errorManifestId: 'err-live-1',
                processId: 'proc-001',
                pipelineStage: 'Enrich',
                errorType: 'AGENT_DAEMON_CRASH_DURING_SESSION',
                errorMessage: 'Daemon crash interrupted the supplier enrichment session.',
                cleanupVerdict: 'leaked',
                severity: 'critical',
                createdAt: '2026-03-28T09:15:00Z',
                resolved: false,
              },
            ],
            nextCursor: null,
            hasMore: false,
          },
          snapshotSeq: 300,
          snapshotAt: '2026-03-28T21:46:00Z',
        },
      });
      return;
    }

    if (body.cmd === 'rerun_from_error') {
      await route.fulfill({
        json: {
          accepted: true,
          visibleAfterSeq: 301,
          jobId: 'rerun-job-1',
        },
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/errors');
  await page.getByRole('button', { name: 'Rerun from error' }).click();
  await expect(page.getByText('Rerun accepted as rerun-job-1.')).toBeVisible();
  await expect(
    page
      .getByRole('listitem')
      .filter({ hasText: 'AGENT_DAEMON_CRASH_DURING_SESSION' })
      .first(),
  ).toBeVisible();
});
