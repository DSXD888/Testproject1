import { Component } from 'react';
import type { ErrorInfo, ReactNode } from 'react';

export interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

/**
 * Catches render-time throws anywhere in the tree so a single broken page
 * shows a recoverable message instead of unmounting the entire window to a
 * blank frame.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Surface the failure in the dev console; later beads can route this to
    // the forensic/observability channel.
    console.error('Desktop UI crashed:', error, info.componentStack);
  }

  private readonly handleReset = (): void => {
    this.setState({ error: null });
  };

  render(): ReactNode {
    const { error } = this.state;
    if (error) {
      return (
        <section className="page page--error-boundary" role="alert">
          <header className="page__header">
            <h2>Something went wrong</h2>
            <p className="page__lede">
              The desktop UI hit an unexpected error and stopped rendering this view.
            </p>
          </header>
          <pre
            style={{
              whiteSpace: 'pre-wrap',
              background: '#1a1a1a',
              color: '#f5f5f5',
              padding: '12px 16px',
              borderRadius: '6px',
              overflowX: 'auto',
            }}
          >
            {error.message}
          </pre>
          <button type="button" onClick={this.handleReset}>
            Try again
          </button>
        </section>
      );
    }

    return this.props.children;
  }
}
