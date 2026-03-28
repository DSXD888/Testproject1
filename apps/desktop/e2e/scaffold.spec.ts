import { expect, test } from '@playwright/test';

test('desktop scaffold boots in browser dev mode', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'Control limited' })).toBeVisible();
  await expect(page.getByText('Contracts route')).toBeVisible();
});
