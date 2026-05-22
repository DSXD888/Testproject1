import { fileURLToPath } from 'node:url';

import { defineConfig } from 'vitest/config';

const biddingRoot = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig({
  root: biddingRoot,
  test: {
    environment: 'node',
    globals: true,
    include: ['tests/**/*.{test,spec}.ts'],
    passWithNoTests: true,
  },
});
