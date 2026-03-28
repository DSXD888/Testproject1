import { fileURLToPath } from 'node:url';

import { defineConfig } from 'vitest/config';

const desktopRoot = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig({
  root: desktopRoot,
  test: {
    environment: 'node',
    globals: true,
    passWithNoTests: true,
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
  },
});
