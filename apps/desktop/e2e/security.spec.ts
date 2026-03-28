import { expect, test } from '@playwright/test';

type InvokeRequest = {
  cmd: string;
  args?: Record<string, unknown>;
};

function processDetailFixture(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    snapshotSeq: 70,
    snapshotAt: '2026-03-28T22:10:00Z',
    data: {
      processId: 'process-sec',
      canonicalProcessId: 'process-sec',
      notices: [
        {
          noticeId: 'notice-1',
          title: 'Security fixture notice',
          buyerName: 'Buyer',
          publishedAt: '2026-03-28T20:00:00Z',
          deadlineAt: null,
          procurementStage: 'tender',
          summary: 'Fixture summary',
        },
      ],
      lots: [],
      candidates: [],
      quotes: [],
      landedCosts: [],
      manifest: {
        markdown: '## Summary\nFixture manifest',
        basisHash: 'basis-sec',
        generatedAt: '2026-03-28T20:05:00Z',
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
        disposition: null,
        isStale: false,
        decidedAt: null,
      },
      errors: [],
      ...overrides,
    },
  };
}

function errorEnvelope(errorMessage: string, errorType = 'AGENT_SKILL_FAILED') {
  return {
    snapshotSeq: 71,
    snapshotAt: '2026-03-28T22:10:00Z',
    data: {
      items: [
        {
          errorManifestId: 'err-sec-1',
          processId: 'process-sec',
          pipelineStage: 'extract',
          errorType,
          errorMessage,
          cleanupVerdict: 'forced',
          severity: 'warning',
          createdAt: '2026-03-28T22:08:00Z',
          resolved: false,
        },
      ],
      nextCursor: null,
      hasMore: false,
    },
  };
}

function sessionListEnvelope(sessionId = 'session-sec', executionMode: 'pty' | 'contract' = 'pty') {
  return {
    snapshotSeq: 72,
    snapshotAt: '2026-03-28T22:10:00Z',
    data: {
      items: [
        {
          sessionId,
          daemonInstanceId: 'daemon-1',
          skillPath: 'skills/procurement-ingestion/fts/SKILL.md',
          skillVersion: '1.0.0',
          executionMode,
          phase: executionMode === 'pty' ? 'active' : 'exited',
          startedAt: '2026-03-28T21:00:00Z',
          endedAt: executionMode === 'pty' ? null : '2026-03-28T21:02:00Z',
          durationMs: executionMode === 'pty' ? null : 120000,
          cleanupVerdict: executionMode === 'pty' ? null : 'clean',
          errorType: null,
          exitCode: executionMode === 'pty' ? null : 0,
          checkpointCount: 1,
          byteCount: 10,
          reattachable: executionMode === 'pty',
        },
      ],
      nextCursor: null,
      hasMore: false,
    },
  };
}

function sessionHistoryEnvelope(text: string, executionMode: 'pty' | 'stdout' = 'pty') {
  return {
    snapshotSeq: 73,
    snapshotAt: '2026-03-28T22:10:01Z',
    data: {
      sessionId: 'session-sec',
      firstSeq: 1,
      lastSeq: 1,
      availability: 'complete',
      frames: [
        {
          kind: executionMode,
          seq: 1,
          text,
        },
      ],
    },
  };
}

test('PW-SEC-001 renders hostile manifest HTML as inert text nodes', async ({ page }) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
    if (body.cmd === 'get_process_detail') {
      await route.fulfill({
        json: processDetailFixture({
          manifest: {
            markdown: '## Summary\n<img src=x onerror=payload-run>\n<svg onload=payload-run></svg>',
            basisHash: 'basis-sec-1',
            generatedAt: '2026-03-28T20:05:00Z',
            sections: ['Summary'],
          },
        }),
      });
      return;
    }
    await route.fallback();
  });

  await page.goto('/process/process-sec');
  await expect(page.getByText('<img src=x onerror=payload-run>')).toBeVisible();
  await expect(page.locator('img')).toHaveCount(0);
  await expect(page.locator('svg')).toHaveCount(0);
});

test('PW-SEC-002 strips dangerous links in manifest and error drill-down views', async ({
  page,
}) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

    if (body.cmd === 'get_process_detail') {
      await route.fulfill({
        json: processDetailFixture({
          manifest: {
            markdown:
              '## Summary\n[safe](https://example.test)\n[bad](javascript:alert(1))\n[worse](data:text/html,<script>alert(1)</script>)',
            basisHash: 'basis-sec-2',
            generatedAt: '2026-03-28T20:05:00Z',
            sections: ['Summary'],
          },
        }),
      });
      return;
    }

    if (body.cmd === 'list_errors') {
      await route.fulfill({
        json: errorEnvelope('[danger](javascript:alert(1)) and [safe](https://example.test)'),
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/process/process-sec');
  await expect(page.locator('a[href="https://example.test"]')).toHaveCount(1);
  await expect(page.locator('a[href^="javascript:"]')).toHaveCount(0);
  await expect(page.locator('a[href^="data:"]')).toHaveCount(0);

  await page.goto('/errors');
  await expect(page.locator('a[href="https://example.test"]')).toHaveCount(1);
  await expect(page.locator('a[href^="javascript:"]')).toHaveCount(0);
});

test('PW-SEC-003 keeps error HTML escaped in both summary and drill-down views', async ({
  page,
}) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
    if (body.cmd === 'list_errors') {
      await route.fulfill({
        json: errorEnvelope('<img src=x onerror=payload-run>'),
      });
      return;
    }
    await route.fallback();
  });

  await page.goto('/errors');
  await expect(page.getByText('<img src=x onerror=payload-run>').first()).toBeVisible();
  await expect(page.locator('img')).toHaveCount(0);
});

test('PW-SEC-004 strips OSC 8 hyperlink escape targets before paint', async ({ page }) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
    if (body.cmd === 'list_sessions') {
      await route.fulfill({ json: sessionListEnvelope('session-sec', 'contract') });
      return;
    }
    if (body.cmd === 'get_session_history') {
      await route.fulfill({
        json: sessionHistoryEnvelope(
          `start \u001b]8;;https://evil.test/hidden\u0007trusted label\u001b]8;;\u0007 end`,
          'stdout',
        ),
      });
      return;
    }
    await route.fallback();
  });

  await page.goto('/sessions/session-sec?initialMode=history');
  await expect(page.getByText('trusted label')).toBeVisible();
  await expect(page.getByText('evil.test')).toHaveCount(0);
});

test('PW-SEC-005 strips OSC 52 clipboard sequences from session output', async ({ page }) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;
    if (body.cmd === 'list_sessions') {
      await route.fulfill({ json: sessionListEnvelope('session-sec', 'contract') });
      return;
    }
    if (body.cmd === 'get_session_history') {
      await route.fulfill({
        json: sessionHistoryEnvelope(
          `clipboard \u001b]52;c;YmFkLXNlY3JldA==\u0007 should not leak`,
          'stdout',
        ),
      });
      return;
    }
    await route.fallback();
  });

  await page.goto('/sessions/session-sec?initialMode=history');
  await expect(page.getByText('should not leak')).toBeVisible();
  await expect(page.getByText('YmFkLXNlY3JldA==')).toHaveCount(0);
});

test('PW-SEC-006 renders hostile strings as text across detail, error, and session surfaces', async ({
  page,
}) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

    if (body.cmd === 'get_process_detail') {
      await route.fulfill({
        json: processDetailFixture({
          notices: [
            {
              noticeId: 'notice-1',
              title: '<marquee>notice</marquee>',
              buyerName: '<b>buyer</b>',
              publishedAt: '2026-03-28T20:00:00Z',
              deadlineAt: null,
              procurementStage: 'tender',
              summary: '<script>summary</script>',
            },
          ],
          candidates: [
            {
              candidateId: 'candidate-1',
              name: '<script>candidate</script>',
              candidateType: 'good',
              specificationText: '<blink>specification</blink>',
              quoteabilityScoreBps: 5000,
            },
          ],
        }),
      });
      return;
    }

    if (body.cmd === 'list_errors') {
      await route.fulfill({
        json: errorEnvelope('<b>error</b>', '<i>TYPE</i>'),
      });
      return;
    }

    if (body.cmd === 'list_sessions') {
      await route.fulfill({ json: sessionListEnvelope() });
      return;
    }

    if (body.cmd === 'get_session_history') {
      await route.fulfill({
        json: sessionHistoryEnvelope('<blink>terminal</blink>'),
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/process/process-sec');
  await expect(page.getByText('<marquee>notice</marquee>')).toBeVisible();
  await expect(page.locator('marquee')).toHaveCount(0);

  await page.goto('/errors');
  await expect(page.getByText('<b>error</b>').first()).toBeVisible();
  await expect(page.locator('b')).toHaveCount(0);

  await page.goto('/sessions/session-sec');
  await expect(page.getByText('<blink>terminal</blink>')).toBeVisible();
  await expect(page.locator('blink')).toHaveCount(0);
});

test('PW-SEC-007 uses bounded rendering markers for oversized manifest and session payloads', async ({
  page,
}) => {
  test.slow();
  const largeManifest = `## Summary\n${'0123456789abcdef'.repeat(700_000)}`;
  const largeReplay = 'line\n'.repeat(100_000);

  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

    if (body.cmd === 'get_process_detail') {
      await route.fulfill({
        json: processDetailFixture({
          manifest: {
            markdown: largeManifest,
            basisHash: 'basis-large',
            generatedAt: '2026-03-28T20:05:00Z',
            sections: ['Summary'],
          },
        }),
      });
      return;
    }

    if (body.cmd === 'list_sessions') {
      await route.fulfill({ json: sessionListEnvelope('session-sec', 'contract') });
      return;
    }

    if (body.cmd === 'get_session_history') {
      await route.fulfill({
        json: sessionHistoryEnvelope(largeReplay, 'stdout'),
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/process/process-sec');
  await expect(page.getByRole('heading', { name: 'Manifest' })).toBeVisible();
  await expect(page.getByText('Manifest truncated to the first')).toBeVisible();

  await page.goto('/sessions/session-sec');
  await expect(page.getByRole('heading', { name: 'Session viewer' })).toBeVisible();
  await expect(page.getByText('Replay frame truncated to')).toBeVisible();
});

test('PW-SEC-008 neutralizes unicode spoofing controls across surfaces', async ({ page }) => {
  await page.route('**/__tauri_invoke__', async (route) => {
    const body = JSON.parse(route.request().postData() ?? '{}') as InvokeRequest;

    if (body.cmd === 'get_process_detail') {
      await route.fulfill({
        json: processDetailFixture({
          canonicalProcessId: `safe\u202Ecod.exe`,
        }),
      });
      return;
    }

    if (body.cmd === 'list_errors') {
      await route.fulfill({
        json: errorEnvelope(`warn\u200Fhidden`),
      });
      return;
    }

    if (body.cmd === 'list_sessions') {
      await route.fulfill({ json: sessionListEnvelope() });
      return;
    }

    if (body.cmd === 'get_session_history') {
      await route.fulfill({
        json: sessionHistoryEnvelope(`session\u2066spoofed`),
      });
      return;
    }

    await route.fallback();
  });

  await page.goto('/process/process-sec');
  await expect(page.getByText('[U+202E]')).toBeVisible();

  await page.goto('/errors');
  await expect(page.getByText('[U+200F]').first()).toBeVisible();

  await page.goto('/sessions/session-sec');
  await expect(page.getByText('[U+2066]').first()).toBeVisible();
});
