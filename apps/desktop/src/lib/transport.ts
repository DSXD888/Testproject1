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
  __DEV_BRIDGE_CALLS__?: Array<{ cmd: string; args?: Record<string, unknown> }>;
  fetch: typeof fetch;
  WebSocket: typeof WebSocket;
  location?: Pick<Location, 'origin' | 'search'>;
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
const TAURI_PARAMS_WRAPPED_COMMANDS = new Set([
  'get_process_detail',
  'get_session_history',
  'trigger_ingestion',
  'trigger_enrichment',
  'capture_decision',
  'get_decision_history',
  'get_active_overrides',
  'capture_clarification',
  'withdraw_clarification',
  'withdraw_decision',
  'trigger_pipeline_rerun',
  'resolve_error',
  'rerun_from_error',
  'retry_job',
  'stop_session',
  'send_session_input',
  'resize_session',
  'subscribe_topic',
  'unsubscribe_topic',
]);
let tauriEventModulePromise:
  | Promise<typeof import('@tauri-apps/api/event')>
  | null = null;

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
  const pageParams = new URLSearchParams(runtime.location?.search ?? '');
  const mode = pageParams.get('mode');
  if (mode) {
    url.searchParams.set('mode', mode);
  }
  if (fromSeq !== undefined) {
    url.searchParams.set('fromSeq', String(fromSeq));
  }
  return url.toString();
}

async function loadTauriEventModule(): Promise<typeof import('@tauri-apps/api/event')> {
  tauriEventModulePromise ??= import('@tauri-apps/api/event');
  return tauriEventModulePromise;
}

function shapeNativeInvokeArgs(
  cmd: string,
  args: Record<string, unknown> | undefined,
): Record<string, unknown> | undefined {
  if (!args) {
    return args;
  }

  if (TAURI_PARAMS_WRAPPED_COMMANDS.has(cmd)) {
    return { params: args };
  }

  return args;
}

function nativeEventNameForTopic(topic: string): string {
  return `procintel/topic/${topic.replace(/[^A-Za-z0-9_:\-\/]/g, '_')}`;
}

function subscribeViaNativeTauriEvents<T>(
  runtime: RuntimeLike,
  topic: string,
  fromSeq: number | undefined,
  onEvent: SubscribeHandler<T>,
): Unsubscribe {
  const nativeInvoke = runtime.__TAURI_INTERNALS__?.invoke;
  if (!nativeInvoke) {
    return () => undefined;
  }

  let active = true;
  let unlisten: Unsubscribe | null = null;

  void (async () => {
    const { listen } = await loadTauriEventModule();
    const detach = await listen<T>(nativeEventNameForTopic(topic), (event) => {
      onEvent(event.payload);
    });

    if (!active) {
      detach();
      return;
    }

    unlisten = detach;

    try {
      await nativeInvoke('subscribe_topic', shapeNativeInvokeArgs('subscribe_topic', {
        topic,
        fromSeq,
      }));
    } catch {
      detach();
      unlisten = null;
    }
  })();

  return () => {
    active = false;
    void nativeInvoke(
      'unsubscribe_topic',
      shapeNativeInvokeArgs('unsubscribe_topic', {
        topic,
        fromSeq,
      }),
    ).catch(() => undefined);
    unlisten?.();
    unlisten = null;
  };
}

export function createTransport(runtime: RuntimeLike): DesktopTransport {
  const nativeInternals = runtime.__TAURI_INTERNALS__;
  if (nativeInternals?.invoke) {
    return {
      invoke: (cmd, args) => nativeInternals.invoke(cmd, shapeNativeInvokeArgs(cmd, args)),
      subscribe: <T = unknown>(
        topic: string,
        fromSeq: number | undefined,
        onEvent: SubscribeHandler<T>,
      ) =>
        nativeInternals.subscribe?.(topic, fromSeq, onEvent) ??
        subscribeViaNativeTauriEvents(runtime, topic, fromSeq, onEvent),
    };
  }

  return {
    invoke: async (cmd, args) => {
      runtime.__DEV_BRIDGE_CALLS__ ??= [];
      runtime.__DEV_BRIDGE_CALLS__.push({ cmd, args });
      const response = await runtime.fetch(DEV_INVOKE_PATH, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Dev-Page-Search': runtime.location?.search ?? '',
        },
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
