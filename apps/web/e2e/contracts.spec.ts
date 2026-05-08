import { expect, test } from '@playwright/test';

test('CI-L4-006 web server starts through the configured Playwright webServer', async ({
  page
}) => {
  await page.goto('/');
  await expect(page.getByTestId('web-shell')).toBeVisible();
});
