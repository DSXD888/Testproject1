import type { ReactElement } from 'react';

import { useControlPlaneState, type RuntimeMode } from './hooks/use-control-plane';
import SessionsPage from './pages/Sessions';

export type AppRoutes =
  | '/'
  | `/process/${string}`
  | '/sessions'
  | `/sessions/${string}`
  | '/errors'
  | '/daemon';

type RouteDescriptor =
  | { kind: 'contracts'; label: string }
  | { kind: 'process-detail'; processId: string; label: string }
  | { kind: 'sessions'; label: string }
  | { kind: 'session-viewer'; sessionId: string; label: string }
  | { kind: 'errors'; label: string }
  | { kind: 'daemon'; label: string }
  | { kind: 'not-found'; label: string };

function resolveRoute(pathname: string): RouteDescriptor {
  if (pathname === '/') {
    return { kind: 'contracts', label: 'Contracts' };
  }

  if (pathname === '/sessions') {
    return { kind: 'sessions', label: 'Sessions' };
  }

  if (pathname === '/errors') {
    return { kind: 'errors', label: 'Errors' };
  }

  if (pathname === '/daemon') {
    return { kind: 'daemon', label: 'Daemon' };
  }

  const processMatch = pathname.match(/^\/process\/(.+)$/);
  if (processMatch) {
    return {
      kind: 'process-detail',
      processId: processMatch[1] ?? '',
      label: 'Process detail',
    };
  }

  const sessionMatch = pathname.match(/^\/sessions\/(.+)$/);
  if (sessionMatch) {
    return {
      kind: 'session-viewer',
      sessionId: sessionMatch[1] ?? '',
      label: 'Session viewer',
    };
  }

  return { kind: 'not-found', label: 'Not found' };
}

function modeBanner(mode: RuntimeMode): { title: string; detail: string } {
  if (mode === 'full control') {
    return {
      title: 'Full control',
      detail: 'All desktop actions are available.',
    };
  }

  if (mode === 'control limited') {
    return {
      title: 'Control limited',
      detail: 'Daemon limited — some operations unavailable.',
    };
  }

  if (mode === 'browse-only') {
    return {
      title: 'Browse-only',
      detail: 'Daemon unavailable — read-only local snapshot.',
    };
  }

  if (mode === 'incompatible') {
    return {
      title: 'Incompatible',
      detail: 'Protocol or schema mismatch detected.',
    };
  }

  return {
    title: 'Recovery required',
    detail: 'Storage is unreadable or still repairing.',
  };
}

function routeCapabilities(mode: RuntimeMode, route: RouteDescriptor): string {
  if (route.kind === 'not-found') {
    return 'Unknown route.';
  }

  if (mode === 'full control') {
    return 'Mutations enabled.';
  }

  if (mode === 'control limited') {
    return 'Mutations may be refused while the daemon is limited.';
  }

  if (mode === 'browse-only') {
    return 'Read-only local snapshot. Triggers and live controls are unavailable.';
  }

  if (mode === 'incompatible') {
    return 'Restart daemon, update desktop, or switch to browse-only.';
  }

  return 'Repair or recover storage before continuing.';
}

function renderRoute(route: RouteDescriptor): ReactElement {
  if (route.kind === 'contracts') {
    return <section data-route="/">Contracts route</section>;
  }

  if (route.kind === 'process-detail') {
    return (
      <section data-route={`/process/${route.processId}`}>
        Process route for {route.processId}
      </section>
    );
  }

  if (route.kind === 'sessions') {
    return <SessionsPage />;
  }

  if (route.kind === 'session-viewer') {
    return (
      <section data-route={`/sessions/${route.sessionId}`}>
        Session route for {route.sessionId}
      </section>
    );
  }

  if (route.kind === 'errors') {
    return <section data-route="/errors">Errors route</section>;
  }

  if (route.kind === 'daemon') {
    return <section data-route="/daemon">Daemon route</section>;
  }

  return <section data-route="not-found">Not found</section>;
}

export interface AppProps {
  pathname?: string;
}

export default function App({ pathname }: AppProps): ReactElement {
  const controlPlane = useControlPlaneState();
  const route = resolveRoute(pathname ?? globalThis.location?.pathname ?? '/');
  const banner = modeBanner(controlPlane.mode);

  return (
    <main data-runtime-mode={controlPlane.mode}>
      <header>
        <h1>{banner.title}</h1>
        <p>{banner.detail}</p>
        <p>{routeCapabilities(controlPlane.mode, route)}</p>
        <ul>
          {controlPlane.state.reasons.map((reason) => (
            <li key={reason}>{reason}</li>
          ))}
        </ul>
      </header>
      {renderRoute(route)}
    </main>
  );
}
