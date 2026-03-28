export interface InvokePayload {
  cmd: string;
  args?: Record<string, unknown>;
}

export interface BridgeErrorPayload {
  code: string;
  message: string;
  retryable: boolean;
  details?: Record<string, unknown> | null;
}

export type InvokeFn = (
  cmd: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

export type SubscribeHandler<T = unknown> = (event: T) => void;
export type Unsubscribe = () => void;

export interface NativeTransportInternals {
  invoke: InvokeFn;
  subscribe?: <T = unknown>(
    topic: string,
    fromSeq: number | undefined,
    onEvent: SubscribeHandler<T>,
  ) => Unsubscribe;
}

export interface RuntimeLike {
  __TAURI_INTERNALS__?: NativeTransportInternals;
  fetch: typeof fetch;
  WebSocket: typeof WebSocket;
  location?: Pick<Location, 'origin'>;
}

export interface DesktopTransport {
  invoke: InvokeFn;
  subscribe: <T = unknown>(
    topic: string,
    fromSeq: number | undefined,
    onEvent: SubscribeHandler<T>,
  ) => Unsubscribe;
}

const DEV_INVOKE_PATH = '/__tauri_invoke__';
const DEV_WS_PATH = '/__ws__';

export function parseBridgeError(value: unknown): BridgeErrorPayload | null {
  if (!value || typeof value !== 'object') {
    return null;
  }

  const candidate = value as Record<string, unknown>;
  if (typeof candidate.code !== 'string' || typeof candidate.message !== 'string') {
    return null;
  }

  return {
    code: candidate.code,
    message: candidate.message,
    retryable:
      typeof candidate.retryable === 'boolean'
        ? candidate.retryable
        : candidate.code === 'transport_unavailable' || candidate.code === 'busy',
    details:
      candidate.details && typeof candidate.details === 'object'
        ? (candidate.details as Record<string, unknown>)
        : null,
  };
}

export function coerceBridgeError(
  value: unknown,
  fallbackMessage = 'Desktop bridge unavailable.',
): BridgeErrorPayload {
  const bridgeError = parseBridgeError(value);
  if (bridgeError) {
    return bridgeError;
  }

  if (value instanceof Error) {
    return {
      code: 'transport_unavailable',
      message: value.message || fallbackMessage,
      retryable: true,
      details: null,
    };
  }

  return {
    code: 'transport_unavailable',
    message: fallbackMessage,
    retryable: true,
    details: null,
  };
}

function buildWsUrl(runtime: RuntimeLike, topic: string, fromSeq?: number): string {
  const origin = runtime.location?.origin ?? 'http://127.0.0.1:5173';
  const url = new URL(DEV_WS_PATH, origin);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  url.searchParams.set('topic', topic);
  if (fromSeq !== undefined) {
    url.searchParams.set('fromSeq', String(fromSeq));
  }
  return url.toString();
}

export function createTransport(runtime: RuntimeLike): DesktopTransport {
  if (runtime.__TAURI_INTERNALS__?.invoke) {
    return {
      invoke: runtime.__TAURI_INTERNALS__.invoke,
      subscribe: <T = unknown>(
        topic: string,
        fromSeq: number | undefined,
        onEvent: SubscribeHandler<T>,
      ) =>
        runtime.__TAURI_INTERNALS__?.subscribe?.(topic, fromSeq, onEvent) ??
        (() => undefined),
    };
  }

  return {
    invoke: async (cmd, args) => {
      const response = await runtime.fetch(DEV_INVOKE_PATH, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ cmd, args }),
      });
      return response.json();
    },
    subscribe: <T = unknown>(
      topic: string,
      fromSeq: number | undefined,
      onEvent: SubscribeHandler<T>,
    ) => {
      const socket = new runtime.WebSocket(buildWsUrl(runtime, topic, fromSeq));
      socket.onmessage = (event) => {
        try {
          onEvent(JSON.parse(String(event.data)) as T);
        } catch {
          // Ignore malformed dev-bridge frames; later beads add explicit diagnostics.
        }
      };
      return () => socket.close();
    },
  };
}

const defaultRuntime = globalThis as RuntimeLike;

export const invoke: InvokeFn = (cmd, args) =>
  createTransport(defaultRuntime).invoke(cmd, args);

export const subscribe = <T = unknown>(
  topic: string,
  fromSeq: number | undefined,
  onEvent: SubscribeHandler<T>,
): Unsubscribe => createTransport(defaultRuntime).subscribe(topic, fromSeq, onEvent);

export const devBridgePaths = {
  invoke: DEV_INVOKE_PATH,
  websocket: DEV_WS_PATH,
};
