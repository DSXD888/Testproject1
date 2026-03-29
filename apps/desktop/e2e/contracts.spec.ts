import { expect, test } from '@playwright/test';

test.describe('contracts pages', () => {
  test('PW-CON-001 list page renders queue rows from one envelope', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByText('Contract queue')).toBeVisible();
    await expect(page.getByTestId('contract-row')).toHaveCount(25);
    await expect(page.getByTestId('contract-row').first()).toContainText('Process process-1');
    await expect(page.getByTestId('contract-row').first()).toContainText('Buyer process-1');
    await expect(page.getByTestId('contract-row').first()).toContainText('Decision: watch');
    await expect(page.getByTestId('contract-row').first()).toContainText('public_web');
    await expect(page.getByTestId('contract-row').first()).toContainText('goods');
  });

  test('PW-CON-002 empty query shows explicit empty state', async ({ page }) => {
    await page.goto('/?sourceId=empty');
    await expect(page.getByTestId('contracts-empty-state')).toBeVisible();
    await expect(page.getByText('synthetic warning banner')).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Next page' })).toBeDisabled();
  });

  test('PW-CON-003 next page uses cursor without duplicates', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Next page' }).click();
    await expect(page.getByTestId('contract-row')).toHaveCount(10);
    await expect(page.getByText('Cursor: terminal-page')).toBeVisible();
    await expect(page.locator('[data-process-id="process-1"]')).toHaveCount(0);
  });

  test('PW-CON-004 refresh keeps selected row stable', async ({ page }) => {
    await page.goto('/');
    const firstCard = page.locator('.contracts-grid__button').nth(2);
    await firstCard.focus();
    await page.getByRole('button', { name: 'Refresh queue' }).click();
    await expect(firstCard).toBeFocused();
  });

  test('PW-CON-005 invalid filter values are rejected before partial render', async ({ page }) => {
    await page.goto('/?pipelineState=unsupported');
    await expect(page.getByRole('alert')).toContainText('Invalid list filter');
    await expect(page.locator('.contracts-grid li')).toHaveCount(0);
  });

  test('PW-CON-006 expired DPS distinction depends on query result, not synthetic rows', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('contract-row')).toHaveCount(25);
    await expect(page.getByText('process-26')).toHaveCount(0);
  });

  test('PW-CON-007 detail page renders the full review packet', async ({ page }) => {
    await page.goto('/process/process-1');
    await expect(page.getByText('Review packet')).toBeVisible();
    await expect(page.locator('.detail-header__rail')).toContainText('Qualified');
    await expect(page.getByText('Steel widgets')).toBeVisible();
    await expect(page.getByText('Lot 1')).toBeVisible();
    await expect(page.getByText('Galvanised steel')).toBeVisible();
    await expect(page.getByText('£12.00 GBP')).toBeVisible();
    await expect(page.getByText('£17.20')).toBeVisible();
    await expect(page.getByText('Cleanup incomplete')).toBeVisible();
  });


  test('PW-CON-008 zero-lot process shows explicit empty-state copy', async ({ page }) => {
    await page.goto('/process/zero-lots');
    await expect(page.getByText('No lot analysis is available for this process yet.')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Approve' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Summary' })).toBeVisible();
  });

  test('PW-CON-009 empty candidates and quotes render placeholders', async ({ page }) => {
    await page.goto('/process/no-candidates');
    await expect(page.getByText('No product candidates have been extracted for this process.')).toBeVisible();
    await expect(page.getByText('No supplier quotes are available yet.')).toBeVisible();
    await expect(page.getByText('0.00%')).toHaveCount(7);
  });

  test('PW-CON-010 detail shows both lot-count and value-weighted gate outcomes', async ({ page }) => {
    await page.goto('/process/process-1');
    await expect(page.getByText('Threshold label')).toBeVisible();
    await expect(page.getByTestId('value-weighted-warning')).toContainText('Coverage below threshold');
    await expect(page.getByText('50.00%')).toBeVisible();
    await expect(page.getByText('42.00%')).toBeVisible();
  });
});
