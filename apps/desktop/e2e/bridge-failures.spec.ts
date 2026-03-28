import { expect, test } from '@playwright/test';

type InvokeRequest = {
  cmd: string;
  args?: Record<string, unknown>;
};

function detailEnvelope(
  processId: string,
  disposition: 'approve' | 'reject' | 'watch' | null,
  snapshotSeq: number,
) {
  return {
    snapshotSeq,
    snapshotAt: '2026-03-28T22:00:00Z',
    data: {
      processId,
      canonicalProcessId: processId,
      notices: [],
      lots: [],
      candidates: [],
      quotes: [],
      landedCosts: [],
      manifest: {
        markdown: '## Summary\nBridge failure fixture',
        basisHash: 'basis-bridge',
        generatedAt: '2026-03-28T21:55:00Z',
        sections: ['Summary'],
      },
      manifestError: null,
      coverage: {
        pricedCoverageBps: 7000,
        landedCostCoverageBps: 6500,
        netQualifiedCoverageBps: 6200,
        pricedValueCoverageBps: 7100,
        landedCostValueCoverageBps: 6100,
        netQualifiedValueCoverageBps: 5200,
        unresolvedMandatoryLots: 0,
      },
      decision: {
        disposition,
        isStale: false,
        decidedAt: disposition ? '2026-03-28T21:59:00Z' : null,
      },
      errors: [],
    },
  };
}

function errorListEnvelope() {
  return {
    snapshotSeq: 55,
    snapshotAt: '2026-03-28T22:00:00Z',
    data: {
      items: [
        {
          errorManifestId: 'err-1',
          processId: 'proc-1',
          pipelineStage: 'enrich',
          errorType: 'AGENT_DAEMON_CRASH_DURING_SESSION',
          errorMessage: 'A durable error manifest is still visible.',
          cleanupVerdict: 'forced',
          severity: 'critical',
          createdAt: '2026-03-28T21:58:00Z',
          resolved: false,
        },
      ],
      nextCursor: null,
      hasMore: false,
    },
  };
}

function sessionListEnvelope(sessionId = 'session-live') {
  return {
    snapshotSeq: 20,
    snapshotAt: '2026-03-28T22:00:00Z',
    data: {
      items: [
        {
          sessionId,
          daemonInstanceId: 'daemon-1',
          skillPath: 'skills/procurement-ingestion/fts/SKILL.md',
          skillVersion: '1.0.0',
          executionMode: 'pty',
          phase: 'active',
          startedAt: '2026-03-28T21:00:00Z',
          endedAt: null,
          durationMs: null,
          cleanupVerdict: null,
          errorType: null,
          exitCode: null,
          checkpointCount: 1,
          byteCount: 10,
          reattachable: true,
        },
      ],
      nextCursor: null,
      hasMore: false,
    },
  };
}

function sessionHistoryEnvelope(text: string) {
  return {
    snapshotSeq: 21,
    snapshotAt: '2026-03-28T22:00:01Z',
    data: {
      sessionId: 'session-live',
      firstSeq: 1,
      lastSeq: 1,
      availability: 'complete',
      frames: [
        {
          kind: 'pty',
          seq: 1,
          text,
        },
      ],
    },
  };
}

test('PW-BRD-001 surfaces transport failure and drops to browse-only mode', async ({
  page,
}) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

    if (body.cmd === 'list_errors') {
      await route.fulfill({ json: errorListEnvelope() });
      return;
    }

    if (body.cmd === 'resolve_error') {
      await route.fulfill({
        json: {
          code: 'transport_unavailable',
          message: 'socket /tmp/procintel/daemon.sock unreachable',
          retryable: true,
        },
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/errors');
  await page.getByRole('button', { name: 'Resolve error' }).click();

  await expect(page.getByText('daemon bridge is unavailable')).toBeVisible();
  await expect(page.getByText('Error resolution committed.')).toHaveCount(0);
  await expect(
    page.getByRole('listitem').filter({ hasText: 'AGENT_DAEMON_CRASH_DURING_SESSION' }),
  ).toHaveCount(1);
});

test('PW-BRD-002 preserves refusal reasons and only shows retry CTA for retryable failures', async ({
  page,
}) => {
  let rerunCalls = 0;

  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

    if (body.cmd === 'list_errors') {
      await route.fulfill({ json: errorListEnvelope() });
      return;
    }

    if (body.cmd === 'rerun_from_error') {
      rerunCalls += 1;
      await route.fulfill({
        json:
          rerunCalls === 1
            ? {
                code: 'conflict',
                message: 'job is not rerunnable because another operator holds the reservation',
                retryable: false,
              }
            : {
                code: 'busy',
                message: 'retry once the daemon job queue clears',
                retryable: true,
              },
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/errors');
  await page.getByRole('button', { name: 'Rerun from error' }).click();
  await expect(page.getByText('blocked by conflict')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Retry rerun' })).toHaveCount(0);

  await page.getByRole('button', { name: 'Rerun from error' }).click();
  await expect(page.getByText('daemon is busy')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Retry rerun' })).toBeVisible();
});

test('PW-BRD-003 surfaces invalid_params without partial session mutation', async ({
  page,
}) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

    if (body.cmd === 'list_sessions') {
      await route.fulfill({ json: sessionListEnvelope() });
      return;
    }

    if (body.cmd === 'get_session_history') {
      await route.fulfill({ json: sessionHistoryEnvelope('ready\n') });
      return;
    }

    if (body.cmd === 'send_session_input') {
      await route.fulfill({
        json: {
          code: 'invalid_params',
          message: 'inputUtf8 must be a non-empty string',
          retryable: false,
        },
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/sessions/session-live');
  await page.getByLabel('PTY input').fill('');
  await page.getByRole('button', { name: 'Send PTY input' }).click();

  await expect(page.getByRole('alert')).toContainText('invalid_params');
  await expect(page.getByText('Sent PTY input to session-live.')).toHaveCount(0);
  await expect(page.getByLabel('PTY input')).toHaveValue('');
});

test('PW-BRD-004 keeps viewer controls disabled after already-terminal no-op responses', async ({
  page,
}) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

    if (body.cmd === 'list_sessions') {
      await route.fulfill({ json: sessionListEnvelope() });
      return;
    }

    if (body.cmd === 'get_session_history') {
      await route.fulfill({ json: sessionHistoryEnvelope('terminal\n') });
      return;
    }

    if (body.cmd === 'resize_session') {
      await route.fulfill({
        json: {
          code: 'already_terminal',
          message: 'session is already archived',
          retryable: false,
        },
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/sessions/session-live');
  await page.getByRole('button', { name: 'Resize 120×40' }).click();

  await expect(page.getByText('already terminal')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Resize 120×40' })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Send PTY input' })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Stop graceful' })).toBeDisabled();
});

test('PW-BRD-005 retries lost acknowledgements with the same idempotency key', async ({
  page,
}) => {
  const keys: string[] = [];
  let detailReads = 0;
  let acknowledged = false;

  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

    if (body.cmd === 'get_process_detail') {
      detailReads += 1;
      await route.fulfill({
        json:
          acknowledged && detailReads >= 2
            ? detailEnvelope('process-1', 'approve', 81)
            : detailEnvelope('process-1', null, 80),
      });
      return;
    }

    if (body.cmd === 'capture_decision') {
      keys.push(String(body.args?.idempotencyKey));
      if (keys.length === 1) {
        await route.fulfill({
          json: {
            code: 'transport_unavailable',
            message: 'ack lost after commit',
            retryable: true,
          },
        });
        return;
      }

      acknowledged = true;
      await route.fulfill({
        json: {
          accepted: true,
          visibleAfterSeq: 81,
          idempotencyKey: body.args?.idempotencyKey,
        },
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/process/process-1');
  await page.getByRole('button', { name: 'Approve' }).click();
  await expect(page.getByRole('alert')).toContainText('same idempotency key');
  await page.getByRole('button', { name: 'Retry last decision' }).click();

  await expect(page.getByText('Decision: approve')).toHaveCount(1);
  expect(keys).toHaveLength(2);
  expect(keys[0]).toBe(keys[1]);
});

test('PW-BRD-006 surfaces reservation conflicts through the error queue instead of stalling', async ({
  page,
}) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

    if (body.cmd === 'list_errors') {
      await route.fulfill({ json: errorListEnvelope() });
      return;
    }

    if (body.cmd === 'rerun_from_error') {
      await route.fulfill({
        json: {
          code: 'conflict',
          message:
            'downstream Agent Mail file_reservation_paths conflict blocked the rerun',
          retryable: false,
        },
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/errors');
  await page.getByRole('button', { name: 'Rerun from error' }).click();

  await expect(page.getByText('file_reservation_paths conflict')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Rerun from error' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Resolve error' })).toBeVisible();
  await expect(
    page.getByRole('listitem').filter({ hasText: 'AGENT_DAEMON_CRASH_DURING_SESSION' }),
  ).toHaveCount(1);
});
