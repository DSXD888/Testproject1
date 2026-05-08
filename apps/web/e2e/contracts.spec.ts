import { expect, test } from '@playwright/test';
import { readdir, readFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  MUTATION_RPC_NAMES,
  READABLE_PROJECTION_TABLES,
  REMOTE_OWNED_TABLES,
} from '@project/contracts-schema';

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
