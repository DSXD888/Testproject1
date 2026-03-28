import { expect, test } from '@playwright/test';

type SessionFixture = {
  sessionId: string;
  daemonInstanceId: string;
  skillPath: string;
  skillVersion: string | null;
  executionMode: 'pty' | 'contract';
  phase: 'created' | 'spawned' | 'active' | 'terminating' | 'exited' | 'reaped' | 'archived';
  startedAt: string | null;
  endedAt: string | null;
  durationMs: number | null;
  cleanupVerdict: 'clean' | 'forced' | 'leaked' | null;
  errorType: string | null;
  exitCode: number | null;
  checkpointCount: number;
  byteCount: number;
  reattachable: boolean;
  jobId?: string | null;
};

function makeSessions(activeOnly: boolean, executionMode: string, snapshotSeq: number): SessionFixture[] {
  const all: SessionFixture[] = [
    {
      sessionId: 'session-live',
      daemonInstanceId: 'daemon-1',
      skillPath: 'skills/procurement-ingestion/fts/SKILL.md',
      skillVersion: '1.0.0',
      executionMode: 'pty',
      phase: 'active',
      startedAt: '2026-03-28T10:00:00Z',
      endedAt: null,
      durationMs: null,
      cleanupVerdict: null,
      errorType: null,
      exitCode: null,
      checkpointCount: 1,
      byteCount: 10,
      reattachable: true,
      jobId: 'job-live',
    },
    {
      sessionId: 'session-failed',
      daemonInstanceId: 'daemon-1',
      skillPath: 'skills/procurement-ingestion/contracts-finder/SKILL.md',
      skillVersion: '1.0.0',
      executionMode: 'contract',
      phase: 'exited',
      startedAt: '2026-03-28T09:00:00Z',
      endedAt: '2026-03-28T09:01:00Z',
      durationMs: 60000,
      cleanupVerdict: 'forced',
      errorType: 'AGENT_CLEANUP_INCOMPLETE',
      exitCode: 1,
      checkpointCount: 0,
      byteCount: 0,
      reattachable: false,
      jobId: 'job-failed',
    },
    {
      sessionId: 'session-browser-auth',
      daemonInstanceId: 'daemon-1',
      skillPath: 'skills/procurement-ingestion/contracts-finder/SKILL.md',
      skillVersion: '1.0.0',
      executionMode: 'pty',
      phase: 'active',
      startedAt: '2026-03-28T10:30:00Z',
      endedAt: null,
      durationMs: null,
      cleanupVerdict: null,
      errorType: null,
      exitCode: null,
      checkpointCount: 2,
      byteCount: 12,
      reattachable: true,
      jobId: 'job-browser-auth',
    },
  ];

  return all.filter((session) => {
    if (activeOnly && session.phase !== 'active') {
      return false;
    }
    if (executionMode !== 'all' && session.executionMode !== executionMode) {
      return false;
    }
    return true;
  });
}

test.describe('sessions page', () => {
  test.beforeEach(async ({ page }) => {
    let listCalls = 0;
    await page.route('**/__tauri_invoke__', async (route) => {
      const request = route.request();
      const body = request.postDataJSON() as {
        cmd: string;
        args?: {
          filters?: { activeOnly?: boolean; executionModes?: string[] };
          jobId?: string;
          sessionId?: string;
          fromSeq?: number;
          toSeq?: number;
          inputUtf8?: string;
          mode?: string;
        };
      };
      if (body.cmd === 'list_sessions') {
        listCalls += 1;
        const activeOnly = body.args?.filters?.activeOnly ?? false;
        const executionMode = body.args?.filters?.executionModes?.[0] ?? 'all';
        const snapshotSeq = listCalls > 2 ? 20 : 10;
        await route.fulfill({
          json: {
            snapshotSeq,
            snapshotAt: '2026-03-28T10:00:00Z',
            data: {
              items: makeSessions(activeOnly, executionMode, snapshotSeq),
              nextCursor: null,
              hasMore: false,
            },
          },
        });
        return;
      }

      if (body.cmd === 'retry_job') {
        await route.fulfill({
          json: {
            accepted: true,
            visibleAfterSeq: 20,
            jobId: body.args?.['jobId'] ?? 'job-failed',
          },
        });
        return;
      }

      if (body.cmd === 'get_session_history') {
<<<<<<< HEAD
        const sessionId = String(body.args?.sessionId ?? '');
        if (sessionId === 'session-live') {
          await route.fulfill({
            json: {
              snapshotSeq: 12,
              snapshotAt: '2026-03-28T10:00:00Z',
              data: {
                sessionId,
                firstSeq: 1,
                lastSeq: 2,
                availability: 'complete',
                frames: [
                  { kind: 'pty', seq: 1, text: 'hello\n' },
                  { kind: 'pty', seq: 2, text: 'world\n' },
                ],
              },
            },
          });
          return;
        }

        if (sessionId === 'session-browser-auth') {
          await route.fulfill({
            json: {
              snapshotSeq: 15,
              snapshotAt: '2026-03-28T10:30:00Z',
              data: {
                sessionId,
                firstSeq: 1,
                lastSeq: 2,
                availability: 'partial',
                frames: [
                  {
                    kind: 'stdout',
                    seq: 1,
                    text: 'Browser auth challenge. reveal_profile_path required.',
                  },
                  {
                    kind: 'stderr',
                    seq: 2,
                    text: 'Use profile path guidance to continue.',
=======
        if (body.args?.sessionId === 'session-live') {
          await route.fulfill({
            json: {
              snapshotSeq: 20,
              snapshotAt: '2026-03-28T10:00:00Z',
              data: {
                sessionId: 'session-live',
                firstSeq: body.args?.fromSeq ?? 6,
                lastSeq: body.args?.toSeq ?? 8,
                availability: 'complete',
                frames: [
                  {
                    seq: 6,
                    kind: 'pty',
                    chunk: [82, 101, 99, 111, 118, 101, 114, 101, 100, 32, 102, 114, 97, 109, 101, 32, 115, 105, 120, 10],
                    ts: '2026-03-28T10:00:06Z',
                  },
                  {
                    seq: 7,
                    kind: 'pty',
                    chunk: [82, 101, 99, 111, 118, 101, 114, 101, 100, 32, 102, 114, 97, 109, 101, 32, 115, 101, 118, 101, 110, 10],
                    ts: '2026-03-28T10:00:07Z',
                  },
                  {
                    seq: 8,
                    kind: 'pty',
                    chunk: [82, 101, 99, 111, 118, 101, 114, 101, 100, 32, 102, 114, 97, 109, 101, 32, 101, 105, 103, 104, 116, 10],
                    ts: '2026-03-28T10:00:08Z',
>>>>>>> 73a88b6 (testproject1-dr6.22: add session viewer and replay flow)
                  },
                ],
              },
            },
          });
          return;
        }

        await route.fulfill({
          json: {
<<<<<<< HEAD
            snapshotSeq: 12,
            snapshotAt: '2026-03-28T10:00:00Z',
            data: {
              sessionId,
              firstSeq: 1,
              lastSeq: 2,
              availability: 'missing',
              frames: [],
=======
            snapshotSeq: 20,
            snapshotAt: '2026-03-28T10:00:00Z',
            data: {
              sessionId: body.args?.sessionId ?? 'session-failed',
              firstSeq: 1,
              lastSeq: 3,
              availability: 'complete',
              frames: [
                {
                  seq: 1,
                  kind: 'stdout',
                  chunk: 'Contracts Finder session booted.',
                  ts: '2026-03-28T09:00:00Z',
                },
                {
                  seq: 2,
                  kind: 'stderr',
                  chunk: 'MFA required in managed browser.',
                  ts: '2026-03-28T09:00:10Z',
                },
                {
                  seq: 3,
                  kind: 'event',
                  event: {
                    type: 'finalResultReady',
                    path: '/tmp/procintel/contracts-finder/result.json',
                  },
                  ts: '2026-03-28T09:02:00Z',
                },
              ],
>>>>>>> 73a88b6 (testproject1-dr6.22: add session viewer and replay flow)
            },
          },
        });
        return;
      }

<<<<<<< HEAD
      if (body.cmd === 'stop_session' || body.cmd === 'send_session_input' || body.cmd === 'resize_session') {
        await route.fulfill({
          json: {
            accepted: true,
            visibleAfterSeq: 25,
            sessionId: body.args?.['sessionId'] ?? null,
=======
      if (
        body.cmd === 'stop_session' ||
        body.cmd === 'send_session_input' ||
        body.cmd === 'resize_session'
      ) {
        await route.fulfill({
          json: {
            accepted: true,
            visibleAfterSeq: 20,
            sessionId: body.args?.sessionId ?? 'session-live',
>>>>>>> 73a88b6 (testproject1-dr6.22: add session viewer and replay flow)
          },
        });
        return;
      }

      await route.fulfill({ json: { ok: false, code: 'unexpected_command' } });
    });
  });

  test('PW-SES-001 renders active and recent sessions from the query envelope', async ({
    page,
  }) => {
    await page.goto('/sessions');
    await expect(page.getByRole('heading', { name: 'Recent sessions' })).toBeVisible();
    await expect(page.locator('[data-session-id="session-live"]')).toContainText(
      'skills/procurement-ingestion/fts/SKILL.md',
    );
    await expect(page.locator('[data-session-id="session-failed"]')).toContainText(
      'AGENT_CLEANUP_INCOMPLETE',
    );
  });

  test('PW-SES-002 keeps failed-before-active sessions visible with canonical phase metadata', async ({
    page,
  }) => {
    await page.goto('/sessions');
    await expect(page.locator('[data-session-id="session-failed"]')).toContainText('Exited');
    await expect(page.locator('[data-session-id="session-failed"]')).toContainText('forced');
  });

  test('PW-SES-005 filters to active sessions only', async ({ page }) => {
    await page.goto('/sessions');
    await page.getByLabel('Active only').check();
    await expect(page.locator('[data-session-id="session-live"]')).toBeVisible();
    await expect(page.locator('[data-session-id="session-failed"]')).toHaveCount(0);
  });

  test('PW-SES-011 retry waits for the visibility fence before reporting success', async ({
    page,
  }) => {
    await page.goto('/sessions');
    await page.getByRole('button', { name: 'Retry first failed job' }).click();
    await expect(page.getByText('Retry queued for session-live')).toBeVisible();
  });

<<<<<<< HEAD
  test('PW-SES-003 opens PTY sessions in live-terminal mode', async ({ page }) => {
    await page.goto('/sessions/session-live');
    await expect(page.getByRole('heading', { name: 'Session viewer' })).toBeVisible();
    await expect(page.getByText('Live terminal output')).toBeVisible();
    await expect(page.getByText('hello')).toBeVisible();
  });

  test('PW-SES-004 opens contract sessions in history mode with input disabled', async ({ page }) => {
    await page.goto('/sessions/session-failed');
    await expect(page.getByText('Historical replay')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Send input' })).toBeDisabled();
    await expect(page.getByText('Input and resize controls are disabled')).toBeVisible();
  });

  test('PW-SES-006 and PW-SES-007 surface graceful and forced stop acknowledgements', async ({
    page,
  }) => {
    await page.goto('/sessions/session-live');
    await page.getByRole('button', { name: 'Stop gracefully' }).click();
    await expect(page.getByText('Graceful stop accepted')).toBeVisible();
    await page.getByRole('button', { name: 'Force stop' }).click();
    await expect(page.getByText('Forced stop accepted')).toBeVisible();
  });

  test('PW-SES-008 and PW-SES-009 handle input gating and browser auth guidance', async ({
    page,
  }) => {
    await page.goto('/sessions/session-browser-auth');
    await expect(page.getByText('Browser-only auth challenge detected')).toBeVisible();
    await page.getByRole('button', { name: 'Send input' }).click();
    await expect(page.getByText('Input sent accepted')).toBeVisible();

    await page.goto('/sessions/session-failed');
    await expect(page.getByRole('button', { name: 'Resize PTY' })).toBeDisabled();
  });

  test('PW-SES-010 allows resize only for attached PTY sessions', async ({ page }) => {
    await page.goto('/sessions/session-live');
    await page.getByRole('button', { name: 'Resize PTY' }).click();
    await expect(page.getByText('Resize applied accepted')).toBeVisible();
  });

  test('PW-SES-012 shows replay state and unresolved gap availability', async ({ page }) => {
    await page.goto('/sessions/session-browser-auth');
    await expect(page.getByText('History availability: partial')).toHaveCount(0);
    await page.getByRole('button', { name: 'Request replay' }).click();
    await expect(page.getByText('Replay partial for session-browser-auth')).toBeVisible();
=======
  test('PW-SES-003 renders live PTY monitoring with replay and auth guidance', async ({
    page,
  }) => {
    await page.goto('/sessions/session-live');
    await expect(page.getByText('Live terminal')).toBeVisible();
    await expect(page.getByText('Browser auth challenge')).toBeVisible();
    await expect(page.locator('p.page__status').filter({ hasText: 'Replay recovered 6:8.' })).toBeVisible();
    await expect(
      page.locator('pre.session-viewer__terminal').filter({ hasText: 'Recovered frame eight' }),
    ).toBeVisible();
  });

  test('PW-SES-004 renders contract sessions in transcript/history mode with input disabled', async ({
    page,
  }) => {
    await page.goto('/sessions/session-failed?initialMode=history');
    await expect(page.getByText('Transcript history')).toBeVisible();
    await expect(page.getByText('Contracts Finder session booted.')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Send PTY input' })).toBeDisabled();
  });

  test('PW-SES-006 supports explicit stop controls', async ({ page }) => {
    await page.goto('/sessions/session-live');
    await page.getByRole('button', { name: 'Stop graceful' }).click();
    await expect(page.getByText('Stop requested (graceful) for session-live.')).toBeVisible();
    await page.getByRole('button', { name: 'Stop force' }).click();
    await expect(page.getByText('Stop requested (force) for session-live.')).toBeVisible();
  });

  test('PW-SES-007 sends PTY input for attached sessions', async ({ page }) => {
    await page.goto('/sessions/session-live');
    await page.getByLabel('PTY input').fill('resume\n');
    await page.getByRole('button', { name: 'Send PTY input' }).click();
    await expect(page.getByText('Sent PTY input to session-live.')).toBeVisible();
  });

  test('PW-SES-008 resizes attached PTY sessions', async ({ page }) => {
    await page.goto('/sessions/session-live');
    await page.getByRole('button', { name: 'Resize 120×40' }).click();
    await expect(page.getByText('Requested 120×40 resize for session-live.')).toBeVisible();
  });

  test('PW-SES-009 surfaces replay availability after gap recovery', async ({ page }) => {
    await page.goto('/sessions/session-live');
    await expect(
      page.locator('p.page__status').filter({ hasText: 'Replay recovered 6:8.' }),
    ).toBeVisible();
  });

  test('PW-SES-010 archived sessions promote to history mode after the archive fence', async ({
    page,
  }) => {
    await page.goto('/sessions/session-failed');
    await expect(page.getByText('Viewer mode')).toBeVisible();
    await expect(page.locator('.meta-card strong').filter({ hasText: 'history' })).toBeVisible();
  });

  test('PW-SES-012 keeps a plain-text transcript alongside the PTY view', async ({
    page,
  }) => {
    await page.goto('/sessions/session-live');
    await page.getByText('Screen-reader transcript').click();
    await expect(
      page.locator('details pre').filter({ hasText: 'Connecting to supplier portal...' }),
    ).toBeVisible();
>>>>>>> 73a88b6 (testproject1-dr6.22: add session viewer and replay flow)
  });
});
