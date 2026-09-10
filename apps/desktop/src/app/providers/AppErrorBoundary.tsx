import { Component } from 'react';
import type { ReactNode } from 'react';

type AppErrorBoundaryProps = {
  children: ReactNode;
};

type AppErrorBoundaryState = {
  hasError: boolean;
};

export class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  public state: AppErrorBoundaryState = { hasError: false };

  public static getDerivedStateFromError(): AppErrorBoundaryState {
    return { hasError: true };
  }

  public render() {
    if (!this.state.hasError) return this.props.children;

    return (
      <main className="flex min-h-screen items-center justify-center bg-canvas p-6 text-text">
        <section
          className="w-full max-w-md rounded-2xl border border-border bg-surface p-6 text-center shadow-lg"
          role="alert"
          aria-live="assertive"
        >
          <h1 className="text-lg font-semibold">Auralis couldn&apos;t display this screen</h1>
          <p className="mt-2 text-sm text-muted">
            Your project data is safe. Restart the application to restore the interface.
          </p>
          <button
            type="button"
            className="mt-5 rounded-lg bg-primary-action px-4 py-2 text-sm font-medium text-primary-foreground"
            onClick={() => globalThis.location.reload()}
          >
            Restart application
          </button>
        </section>
      </main>
    );
  }
}
