// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { Mock } from 'vitest';
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
        <TestComponent
          onContext={(ctx) => {
            contextValue = ctx;
          }}
        />
      </ProjectProvider>,
    );

    // Set a project and start deleting it
    act(() => {
      contextValue.setProjectId('p1');
    });

    // Let the effect run and register the listener
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
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

  it('invalidates in-flight fetches when deletion begins', async () => {
    let contextValue: any;
    let eventCallback: any;
    (listen as any).mockImplementation((event: string, cb: any) => {
      if (event === 'project-updated') eventCallback = cb;
      return Promise.resolve(() => {});
    });

    let resolveGetProject: (val: any) => void = () => {};
    const getProjectPromise = new Promise((resolve) => {
      resolveGetProject = resolve;
    });

    (invoke as Mock).mockReturnValueOnce(getProjectPromise);

    render(
      <ProjectProvider>
        <TestComponent
          onContext={(ctx) => {
            contextValue = ctx;
          }}
        />
      </ProjectProvider>,
    );

    act(() => {
      contextValue.setProjectId('p1');
      contextValue.setProject({ id: 'p1', title: 'Test Project' } as any);
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // Simulate event while project is active (triggering a fetch)
    act(() => {
      void eventCallback({ payload: { projectId: 'p1' } });
    });

    expect(invoke).toHaveBeenCalledTimes(1);

    // Now start project deletion (should increment generation token)
    act(() => {
      contextValue.beginProjectDeletion('p1');
    });

    // Complete the deletion lifecycle: clear context, finish deletion
    act(() => {
      contextValue.setProjectId(null);
      contextValue.setProject(null);
      contextValue.finishProjectDeletion('p1');
    });

    // Resolve the in-flight get_project_cmd call
    await act(async () => {
      resolveGetProject({ id: 'p1', title: 'Resurrected' });
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // Verify that the stale project did not resurrect
    expect(contextValue.project).toBeNull();
  });
});
