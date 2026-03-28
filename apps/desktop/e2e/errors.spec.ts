import { expect, test } from '@playwright/test';

test('PW-ERR-001 lists unresolved manifests with stable severity', async ({ page }) => {
  await page.goto('/errors');

  await expect(page.getByRole('heading', { name: 'Errors' })).toBeVisible();
  await expect(
    page
      .getByRole('listitem')
      .filter({ hasText: 'AGENT_DAEMON_CRASH_DURING_SESSION' })
      .first(),
  ).toBeVisible();
  await expect(page.getByText('Critical', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('Process', { exact: true }).first()).toBeVisible();
});

test('PW-ERR-004 keeps drill-down content sanitized', async ({ page }) => {
  await page.goto('/errors?errorsFixture=hostile');

  await page.getByRole('button', { name: 'View details' }).click();
  await expect(page.getByLabel('Error details')).toContainText(
    '<svg onload=payload-run>alert(1)</svg>',
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
          reads === 1
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
                snapshotSeq: reads === 1 ? 200 : 201,
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
