import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { AppSidebar } from './AppSidebar';

vi.mock('./ProjectListPanel', () => ({
  ProjectListPanel: ({ height }: { height: number }) => (
    <output data-testid="projects-height">{height}</output>
  ),
}));

vi.mock('./PipelinePanel', () => ({
  PipelinePanel: ({ onStep }: { onStep: (step: 'source' | 'subtitles') => void }) => (
    <button type="button" onClick={() => onStep('subtitles')}>
      Open subtitles
    </button>
  ),
}));

vi.mock('./PrimaryNavigation', () => ({
  PrimaryNavigation: ({
    onHome,
    onProject,
    onSettings,
  }: {
    onHome: () => void;
    onProject: () => void;
    onSettings: () => void;
  }) => (
    <nav aria-label="Test navigation">
      <button type="button" onClick={onHome}>
        Projects
      </button>
      <button type="button" onClick={onProject}>
        Workspace
      </button>
      <button type="button" onClick={onSettings}>
        Settings
      </button>
    </nav>
  ),
}));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('AppSidebar', () => {
  it('forwards navigation and pipeline actions', () => {
    const handlers = createHandlers();
    renderSidebar(handlers);

    fireEvent.click(screen.getByRole('button', { name: 'Projects' }));
    fireEvent.click(screen.getByRole('button', { name: 'Workspace' }));
    fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    fireEvent.click(screen.getByRole('button', { name: 'Open subtitles' }));

    expect(handlers.onHome).toHaveBeenCalledOnce();
    expect(handlers.onProject).toHaveBeenCalledOnce();
    expect(handlers.onSettings).toHaveBeenCalledOnce();
    expect(handlers.onStep).toHaveBeenCalledWith('subtitles');
  });

  it('resizes by pointer and detaches global drag listeners after pointer release', () => {
    const removeListener = vi.spyOn(window, 'removeEventListener');
    renderSidebar();
    const separator = screen.getByRole('separator', {
      name: 'Resize projects and pipeline panels',
    });
    Object.defineProperty(separator.parentElement, 'clientHeight', {
      configurable: true,
      value: 500,
    });

    fireEvent.pointerDown(separator, { clientY: 100 });
    fireEvent.pointerMove(window, { clientY: 180 });
    expect(screen.getByTestId('projects-height').textContent).toBe('270');

    fireEvent.pointerUp(window);
    fireEvent.pointerMove(window, { clientY: 240 });

    expect(screen.getByTestId('projects-height').textContent).toBe('270');
    expect(removeListener.mock.calls.some(([event]) => event === 'pointermove')).toBe(true);
    expect(removeListener.mock.calls.some(([event]) => event === 'pointerup')).toBe(true);
  });

  it('detaches active drag listeners when the sidebar unmounts', () => {
    const removeListener = vi.spyOn(window, 'removeEventListener');
    const { unmount } = renderSidebar();
    const separator = screen.getByRole('separator', {
      name: 'Resize projects and pipeline panels',
    });

    fireEvent.pointerDown(separator, { clientY: 100 });
    unmount();

    expect(removeListener.mock.calls.some(([event]) => event === 'pointermove')).toBe(true);
    expect(removeListener.mock.calls.some(([event]) => event === 'pointerup')).toBe(true);
  });
});

function createHandlers() {
  return {
    onHome: vi.fn(),
    onProject: vi.fn(),
    onSettings: vi.fn(),
    onStep: vi.fn(),
  };
}

function renderSidebar(handlers = createHandlers()) {
  return render(
    <AppSidebar
      currentView="home"
      pipelineStep="source"
      hasProject
      pipelineStatus={{ source: 'completed', subtitles: 'running' }}
      {...handlers}
    />,
  );
}
