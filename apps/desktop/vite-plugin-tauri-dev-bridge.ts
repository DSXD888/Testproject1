import type { IncomingMessage, ServerResponse } from 'node:http';

import type { Plugin, ViteDevServer } from 'vite';

import { devBridgePaths, type InvokePayload } from './src/lib/transport';

export interface DevBridgeOptions {
  invokeHandler?: (payload: InvokePayload) => Promise<unknown>;
  websocketHandler?: (request: IncomingMessage) => void;
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
      server.middlewares.use(async (request, response, next) => {
        const pathname = request.url ? new URL(request.url, 'http://127.0.0.1').pathname : '';

        if (request.method !== 'POST' || pathname !== devBridgePaths.invoke) {
          next();
          return;
        }

        try {
          const payload = await readJsonBody(request);
          const result = options.invokeHandler
            ? await options.invokeHandler(payload)
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

      server.httpServer?.on('upgrade', (request) => {
        const pathname = request.url ? new URL(request.url, 'http://127.0.0.1').pathname : '';
        if (pathname === devBridgePaths.websocket) {
          options.websocketHandler?.(request);
        }
      });
    },
  };
}
