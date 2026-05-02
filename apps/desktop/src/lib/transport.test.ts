import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

import { describe, expect, it, vi } from 'vitest';

import { createTransport, devBridgePaths } from './transport';
import { tauriDevBridge } from '../../vite-plugin-tauri-dev-bridge';

const tauriUnlisten = vi.fn();
const tauriListen = vi.fn(async (_event: string, _handler: (event: { payload: unknown }) => void) => tauriUnlisten);

vi.mock('@tauri-apps/api/event', () => ({
  listen: tauriListen,
}));

class MockWebSocket {
  static instances: MockWebSocket[] = [];

  onmessage: ((event: { data: string }) => void) | null = null;
  closed = false;

  constructor(public readonly url: string) {
    MockWebSocket.instances.push(this);
  }

  close(): void {
    this.closed = true;
  }
}

describe('transport shim', () => {
  it('TS-TRN-001 selects native transport when Tauri internals exist', async () => {
    const invoke = vi.fn().mockResolvedValue({ ok: true });
    const subscribe = vi.fn().mockReturnValue(() => undefined);
    const fetch = vi.fn();

    const transport = createTransport({
      __TAURI_INTERNALS__: { invoke, subscribe },
      fetch,
      WebSocket: MockWebSocket as unknown as typeof WebSocket,
      location: { origin: 'http://127.0.0.1:5173', search: '' },
    });

    await transport.invoke('list_processes', { limit: 10 });
    await transport.invoke('capture_decision', { processId: 'proc-1' });
    transport.subscribe('sessions', 12, () => undefined);

    expect(invoke).toHaveBeenCalledWith('list_processes', { limit: 10 });
    expect(invoke).toHaveBeenCalledWith('capture_decision', {
      params: { processId: 'proc-1' },
    });
    expect(subscribe).toHaveBeenCalledWith('sessions', 12, expect.any(Function));
    expect(fetch).not.toHaveBeenCalled();
  });

  it('TS-TRN-002 uses HTTP and WebSocket fallback in browser dev mode', async () => {
    const fetch = vi.fn().mockResolvedValue({
      json: async () => ({ accepted: true }),
    });

    const transport = createTransport({
      fetch,
      WebSocket: MockWebSocket as unknown as typeof WebSocket,
      location: { origin: 'http://127.0.0.1:5173', search: '' },
    });

    await transport.invoke('capture_decision', { processId: 'proc-1' });
    const unsubscribe = transport.subscribe('sessions', 7, () => undefined);

    expect(fetch).toHaveBeenCalledWith(devBridgePaths.invoke, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Dev-Page-Search': '',
      },
      body: JSON.stringify({
        cmd: 'capture_decision',
        args: { processId: 'proc-1' },
      }),
    });
    expect(MockWebSocket.instances.at(-1)?.url).toContain('/__ws__?topic=sessions&fromSeq=7');
    unsubscribe();
    expect(MockWebSocket.instances.at(-1)?.closed).toBe(true);
  });

  it('TS-TRN-002b uses Tauri event subscriptions when native invoke exists without a native subscribe helper', async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const fetch = vi.fn();

    const transport = createTransport({
      __TAURI_INTERNALS__: { invoke },
      fetch,
      WebSocket: MockWebSocket as unknown as typeof WebSocket,
      location: { origin: 'http://127.0.0.1:5173', search: '' },
    });

    const unsubscribe = transport.subscribe('daemon.health', undefined, () => undefined);
    await vi.dynamicImportSettled();

    expect(tauriListen).toHaveBeenCalledWith('procintel/topic/daemon_health', expect.any(Function));
    expect(invoke).toHaveBeenCalledWith('subscribe_topic', {
      params: {
        topic: 'daemon.health',
        fromSeq: undefined,
      },
    });

    unsubscribe();
    await Promise.resolve();

    expect(invoke).toHaveBeenCalledWith('unsubscribe_topic', {
      params: {
        topic: 'daemon.health',
        fromSeq: undefined,
      },
    });
    expect(tauriUnlisten).toHaveBeenCalled();
    expect(fetch).not.toHaveBeenCalled();
  });

  it('TS-TRN-003 keeps runtime bridge selection centralized in transport.ts', async () => {
    const transportSource = await readFile(fileURLToPath(new URL('./transport.ts', import.meta.url)), 'utf8');
    const uiIndexSource = await readFile(
      fileURLToPath(new URL('../../../../packages/ui/index.ts', import.meta.url)),
      'utf8',
    );

    expect(transportSource).toContain('__TAURI_INTERNALS__');
    expect(uiIndexSource).not.toContain('__TAURI_INTERNALS__');
    expect(uiIndexSource).not.toContain('/__tauri_invoke__');
  });

  it('TS-TRN-004 keeps the dev bridge serve-only and production-excluded', async () => {
    const plugin = tauriDevBridge();
    const viteConfigModule = await import('../../vite.config');
    const productionConfig = viteConfigModule.default({
      command: 'build',
      mode: 'production',
    });
    const developmentConfig = viteConfigModule.default({
      command: 'serve',
      mode: 'development',
    });
    const productionPluginNames = (productionConfig.plugins ?? []).flatMap((entry) =>
      entry && typeof entry === 'object' && 'name' in entry ? [String(entry.name)] : [],
    );
    const developmentPluginNames = (developmentConfig.plugins ?? []).flatMap((entry) =>
      entry && typeof entry === 'object' && 'name' in entry ? [String(entry.name)] : [],
    );

    expect(plugin.apply).toBe('serve');
    expect(productionPluginNames).not.toContain('tauri-dev-bridge');
    expect(developmentPluginNames).toContain('tauri-dev-bridge');
  });
});
