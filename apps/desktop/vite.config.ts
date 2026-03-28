import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import { tauriDevBridge } from './vite-plugin-tauri-dev-bridge';

const packageUiAlias = fileURLToPath(new URL('../../packages/ui/index.ts', import.meta.url));

export default defineConfig(({ mode }) => ({
  plugins: [react(), mode === 'development' ? tauriDevBridge() : undefined].filter(
    Boolean,
  ),
  resolve: {
    alias: {
      '@project/ui': packageUiAlias,
    },
  },
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
  },
  preview: {
    host: '127.0.0.1',
    port: 4173,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
}));
