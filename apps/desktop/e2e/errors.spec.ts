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
