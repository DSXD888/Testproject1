import type { IncomingMessage, ServerResponse } from 'node:http';

import type { Plugin, ViteDevServer } from 'vite';
import { WebSocketServer, type WebSocket } from 'ws';

import { devBridgePaths, type InvokePayload } from './src/lib/transport';

export interface DevBridgeInvokeContext {
  payload: InvokePayload;
  request: IncomingMessage;
  url: URL;
}

export interface DevBridgeWebSocketContext {
  request: IncomingMessage;
  socket: WebSocket;
  url: URL;
}

export interface DevBridgeOptions {
  invokeHandler?: (context: DevBridgeInvokeContext) => Promise<unknown>;
  websocketHandler?: (context: DevBridgeWebSocketContext) => void;
}

async function readJsonBody(request: IncomingMessage): Promise<InvokePayload> {
  const chunks: Buffer[] = [];
  for await (const chunk of request) {
    chunks.push(Buffer.from(chunk));
  }

  try {
    return JSON.parse(Buffer.concat(chunks).toString('utf8')) as InvokePayload;
  } catch (error) {
    throw new Error(
      error instanceof Error ? `invalid bridge payload: ${error.message}` : 'invalid bridge payload',
    );
  }
}

function writeJson(response: ServerResponse, statusCode: number, payload: unknown): void {
  response.statusCode = statusCode;
  response.setHeader('Content-Type', 'application/json');
  response.end(JSON.stringify(payload));
}

export function tauriDevBridge(options: DevBridgeOptions = {}): Plugin {
  return {
    name: 'tauri-dev-bridge',
    apply: 'serve',
    configureServer(server: ViteDevServer) {
      const websocketServer = new WebSocketServer({ noServer: true });

      server.middlewares.use(async (request, response, next) => {
        const url = new URL(request.url ?? '/', 'http://127.0.0.1');
        const pathname = url.pathname;

        if (request.method !== 'POST' || pathname !== devBridgePaths.invoke) {
          next();
          return;
        }

        try {
          const payload = await readJsonBody(request);
          const result = options.invokeHandler
            ? await options.invokeHandler({ payload, request, url })
            : {
                ok: false,
                code: 'dev_bridge_unconfigured',
                cmd: payload.cmd,
              };
          writeJson(response, 200, result);
        } catch (error) {
          writeJson(response, 400, {
            ok: false,
            code: 'invalid_json',
            message: error instanceof Error ? error.message : 'invalid bridge payload',
          });
        }
      });

      server.httpServer?.on('upgrade', (request, socket, head) => {
        const url = new URL(request.url ?? '/', 'http://127.0.0.1');
        if (url.pathname === devBridgePaths.websocket) {
          websocketServer.handleUpgrade(request, socket, head, (client: WebSocket) => {
            options.websocketHandler?.({
              request,
              socket: client,
              url,
            });
          });
          return;
        }
      });
    },
  };
}
