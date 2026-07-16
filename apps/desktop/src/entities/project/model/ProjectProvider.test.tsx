// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, act } from '@testing-library/react';
import { ProjectProvider } from './ProjectProvider';
import { useProjectContext } from './useProjectContext';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@/shared/api/tauri';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('@/shared/api/tauri', () => ({
  invoke: vi.fn(),
}));

const TestComponent = ({ onContext }: { onContext: (ctx: any) => void }) => {
  const ctx = useProjectContext();
  onContext(ctx);
  return null;
};

describe('ProjectProvider', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('ignores project-updated event if deletingProjectId matches', async () => {
    let contextValue: any;
    
    // Setup listener mock to capture the callback
    let eventCallback: any;
    (listen as any).mockImplementation((event: string, cb: any) => {
      if (event === 'project-updated') eventCallback = cb;
      return Promise.resolve(() => {});
    });

    render(
      <ProjectProvider>
        <TestComponent onContext={(ctx) => { contextValue = ctx; }} />
      </ProjectProvider>
    );

    // Set a project and start deleting it
    act(() => {
      contextValue.setProjectId('p1');
    });

    // Let the effect run and register the listener
    await act(async () => {
      await new Promise(resolve => setTimeout(resolve, 0));
    });

    act(() => {
      contextValue.beginProjectDeletion('p1');
    });

    // Simulate event
    expect(eventCallback).toBeDefined();
    await act(async () => {
      await eventCallback({ payload: { projectId: 'p1' } });
    });

    // Invoke should not be called because it's currently deleting
    expect(invoke).not.toHaveBeenCalled();
  });
});
