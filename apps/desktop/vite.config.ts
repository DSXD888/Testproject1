import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';
import { tauriDevBridge } from './vite-plugin-tauri-dev-bridge';

export default defineConfig(({ mode }) => ({
  plugins: [react(), mode === 'development' ? tauriDevBridge() : undefined].filter(
    Boolean,
  ),
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
