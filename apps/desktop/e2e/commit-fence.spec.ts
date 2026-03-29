import { expect, test } from '@playwright/test';

test.describe('commit fence', () => {
  test('PW-FEN-003 capture-decision keeps the old state until the fence clears', async ({ page }) => {
    await page.goto('/process/process-1?decisionFixture=delayed-fence-commit&mode=full-control');
    await expect(page.getByText('Decision: watch')).toBeVisible();
    await page.getByRole('button', { name: 'Approve' }).click();
    await expect(page.getByText('Decision: approve')).toBeVisible();
  });

  test('PW-FEN-006 rollback-like failures do not advance the visible decision', async ({ page }) => {
    await page.goto('/process/process-1?decisionFixture=rollback-failure&mode=full-control');
    await page.getByRole('button', { name: 'Reject' }).click();
    await expect(page.getByRole('alert')).toContainText(
      'rollback prevented decision persistence',
    );
    await expect(page.getByText('Decision: watch')).toBeVisible();
  });
});
