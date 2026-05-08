import { expect, test } from '@playwright/test';
import { readdir, readFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  MUTATION_RPC_NAMES,
  READABLE_PROJECTION_TABLES,
  REMOTE_OWNED_TABLES,
} from '@project/contracts-schema';
import { buildSyncFreshnessVm } from '@project/ui';
import {
  PRIVILEGED_TOKEN_NAME_PATTERN,
  viewerCredentialNames,
} from './fixtures/supabase-auth';
import {
  FRESHNESS_NOW,
  freshnessProjectionSeeds,
} from './fixtures/projection-seed';

const WEB_ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const REPO_ROOT = dirname(dirname(WEB_ROOT));

async function listSourceFiles(dir: string): Promise<string[]> {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const path = join(dir, entry.name);

      if (entry.isDirectory()) {
        return listSourceFiles(path);
      }

      return ['.ts', '.tsx'].some((extension) => entry.name.endsWith(extension))
        ? [path]
        : [];
    }),
  );

  return files.flat();
}

test('CI-L4-006 web server starts through the configured Playwright webServer', async ({
  page
}) => {
  await page.goto('/');
  await expect(page.getByTestId('web-shell')).toBeVisible();
});

test('L4-T-contracts_spec-020 keeps Supabase behind the web boundary', async () => {
  const srcFiles = await listSourceFiles(join(WEB_ROOT, 'src'));
  const directSupabaseImports: string[] = [];

  for (const filePath of srcFiles) {
    const source = await readFile(filePath, 'utf8');

    if (source.includes('@supabase/supabase-js')) {
      directSupabaseImports.push(relative(WEB_ROOT, filePath));
    }
  }

  expect(directSupabaseImports).toEqual(['src/lib/supabase.ts']);
  expect(await readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8')).toContain(
    '@project/contracts-schema',
  );
});

test('L4-T-contracts_spec-013-015 exposes viewer-only schema and env names', async () => {
  const boundarySource = await readFile(join(WEB_ROOT, 'src/lib/supabase.ts'), 'utf8');
  const sourceFiles = await listSourceFiles(join(WEB_ROOT, 'src'));
  const sourceText = (
    await Promise.all(sourceFiles.map((filePath) => readFile(filePath, 'utf8')))
  ).join('\n');

  expect(boundarySource).toContain('NEXT_PUBLIC_SUPABASE_URL');
  expect(boundarySource).toContain('NEXT_PUBLIC_SUPABASE_ANON_KEY');
  expect(boundarySource).not.toMatch(/service.?role|admin.?key|operator.?token|daemon.?token/i);
  expect(sourceText).not.toMatch(/127\.0\.0\.1:\d+|localhost:\d+|tauri|sqlite|ipc/i);
  expect(READABLE_PROJECTION_TABLES).toContain('viewer_sync_status');
  expect(READABLE_PROJECTION_TABLES).toContain('operator_decision_summaries');
  expect(REMOTE_OWNED_TABLES).toEqual([
    'shared_company_profile',
    'shared_source_configs',
    'operator_decision_heads',
  ]);
  expect(MUTATION_RPC_NAMES).toEqual(['apply_push_batch', 'update_operator_decision']);
  expect(relative(REPO_ROOT, join(WEB_ROOT, 'src/lib/supabase.ts'))).toBe(
    'apps/web/src/lib/supabase.ts',
  );
});

test('L4-T-contracts_spec-005-007 maps viewer_sync_status to freshness states', () => {
  const states = freshnessProjectionSeeds.map((seed) =>
    buildSyncFreshnessVm({
      syncHealth: seed.syncHealth,
      lastSuccessAt: seed.lastPushSuccessAt,
      now: FRESHNESS_NOW,
    }),
  );

  expect(states).toEqual([
    {
      label: 'Data current',
      tone: 'good',
      tooltip: 'Last successful sync: 2026-03-28T18:29:15Z',
    },
    {
      label: 'Updates delayed -- data may be behind desktop',
      tone: 'warn',
      tooltip: 'Last successful sync: 2026-03-28T18:27:30Z',
    },
    {
      label: 'Sync offline -- showing last known data',
      tone: 'bad',
      tooltip: 'Last successful sync: 2026-03-28T18:29:15Z',
    },
  ]);
});

test('L4-T-contracts_spec-016 keeps UI and web bundle boundaries read-only', async () => {
  const uiFiles = await listSourceFiles(join(REPO_ROOT, 'packages/ui'));
  const webFiles = await listSourceFiles(join(WEB_ROOT, 'src'));
  const uiSource = (
    await Promise.all(uiFiles.map((filePath) => readFile(filePath, 'utf8')))
  ).join('\n');
  const webSource = (
    await Promise.all(webFiles.map((filePath) => readFile(filePath, 'utf8')))
  ).join('\n');

  expect(uiSource).not.toMatch(/@tauri-apps\/api|apps\/desktop|crates\/daemon|supabase|transport/i);
  expect(viewerCredentialNames()).toEqual([
    'NEXT_PUBLIC_SUPABASE_URL',
    'NEXT_PUBLIC_SUPABASE_ANON_KEY',
  ]);
  expect(viewerCredentialNames().join('\n')).not.toMatch(PRIVILEGED_TOKEN_NAME_PATTERN);
  expect(webSource).not.toMatch(PRIVILEGED_TOKEN_NAME_PATTERN);
});
