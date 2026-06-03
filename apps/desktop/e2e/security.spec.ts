import { expect, test } from '../../e2e-playwright-compat';

test('PW-SEC-001 renders hostile manifest HTML as inert text nodes', async ({ page }) => {
  await page.goto('/process/process-sec?securityFixture=manifest-html');
  await expect(page.getByText('<img src=x onerror=payload-run>')).toBeVisible();
  await expect(page.locator('img')).toHaveCount(0);
  await expect(page.locator('svg')).toHaveCount(0);
});

test('PW-SEC-002 strips dangerous links in manifest and error drill-down views', async ({
  page,
}) => {
  await page.goto('/process/process-sec?securityFixture=dangerous-links');
  await expect(page.locator('a[href="https://example.test"]')).toHaveCount(1);
  await expect(page.locator('a[href^="javascript:"]')).toHaveCount(0);
  await expect(page.locator('a[href^="data:"]')).toHaveCount(0);

  await page.goto('/errors?securityFixture=dangerous-links');
  await expect(page.locator('a[href="https://example.test"]')).toHaveCount(1);
  await expect(page.locator('a[href^="javascript:"]')).toHaveCount(0);
});

test('PW-SEC-003 keeps error HTML escaped in both summary and drill-down views', async ({
  page,
}) => {
  await page.goto('/errors?securityFixture=error-html');
  await expect(page.getByText('<img src=x onerror=payload-run>').first()).toBeVisible();
  await expect(page.locator('img')).toHaveCount(0);
});

test('PW-SEC-004 strips OSC 8 hyperlink escape targets before paint', async ({ page }) => {
  await page.goto('/sessions/session-sec?initialMode=history&securityFixture=osc8');
  await expect(page.getByText('trusted label')).toBeVisible();
  await expect(page.getByText('evil.test')).toHaveCount(0);
});

test('PW-SEC-005 strips OSC 52 clipboard sequences from session output', async ({ page }) => {
  await page.goto('/sessions/session-sec?initialMode=history&securityFixture=osc52');
  await expect(page.getByText('should not leak')).toBeVisible();
  await expect(page.getByText('YmFkLXNlY3JldA==')).toHaveCount(0);
});

test('PW-SEC-006 renders hostile strings as text across detail, error, and session surfaces', async ({
  page,
}) => {
  await page.goto('/process/process-sec?securityFixture=hostile-strings');
  await expect(page.getByText('<marquee>notice</marquee>')).toBeVisible();
  await expect(page.locator('marquee')).toHaveCount(0);

  await page.goto('/errors?securityFixture=hostile-strings');
  await expect(page.getByText('<b>error</b>').first()).toBeVisible();
  await expect(page.locator('b')).toHaveCount(0);

  await page.goto('/sessions/session-sec?initialMode=history&securityFixture=hostile-strings');
  await expect(page.getByText('<blink>terminal</blink>')).toBeVisible();
  await expect(page.locator('blink')).toHaveCount(0);
});

test('PW-SEC-007 uses bounded rendering markers for oversized manifest and session payloads', async ({
  page,
}) => {
  test.slow();
  await page.goto('/process/process-sec?securityFixture=large-payload');
  await expect(page.getByRole('heading', { name: 'Manifest' })).toBeVisible();
  await expect(page.getByText('Manifest truncated to the first')).toBeVisible();

  await page.goto('/sessions/session-sec?initialMode=history&securityFixture=large-payload');
  await expect(page.getByRole('heading', { name: 'Transcript history' })).toBeVisible();
  await expect(page.getByText('Showing the most recent')).toBeVisible();
});

test('PW-SEC-008 neutralizes unicode spoofing controls across surfaces', async ({ page }) => {
  await page.goto('/process/process-sec?securityFixture=unicode-spoof');
  await expect(page.getByText('[U+202E]')).toBeVisible();

  await page.goto('/errors?securityFixture=unicode-spoof');
  await expect(page.getByText('[U+200F]').first()).toBeVisible();

  await page.goto('/sessions/session-sec?initialMode=history&securityFixture=unicode-spoof');
  await expect(page.getByText('[U+2066]').first()).toBeVisible();
});
