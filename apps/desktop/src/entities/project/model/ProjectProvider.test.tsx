// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { Mock } from 'vitest';
import { render, act } from '@testing-library/react';
import { ProjectProvider } from './ProjectProvider';
import { useProjectContext } from './useProjectContext';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@/shared/api/tauri';
import { projectRemoved, projectUpdated } from './projectChanges';

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

  it('applies confirmed local updates and rejects an older backend event response', async () => {
    let contextValue: any;
    let eventCallback: any;
    vi.mocked(listen).mockImplementation((_event, callback) => {
      eventCallback = callback;
      return Promise.resolve(() => {});
    });
    let resolve!: (value: any) => void;
    (invoke as Mock).mockReturnValueOnce(
      new Promise((done) => {
        resolve = done;
      }),
    );
    const view = render(
      <ProjectProvider>
        <TestComponent
          onContext={(value) => {
            contextValue = value;
          }}
        />
      </ProjectProvider>,
    );
    await act(async () => {
      contextValue.setProject({ id: 'p1', title: 'Test Project' });
    });
    act(() => {
      void eventCallback({ payload: { projectId: 'p1' } });
    });
    act(() => projectUpdated({ id: 'p1', title: 'Renamed' } as any));
    await act(async () => resolve({ id: 'p1', title: 'Old snapshot' }));
    expect(contextValue.project.title).toBe('Renamed');
    act(() => projectRemoved('other'));
    expect(contextValue.projectId).toBe('p1');
    act(() => projectRemoved('p1'));
    expect(contextValue.projectId).toBeNull();
    expect(contextValue.project).toBeNull();
    view.unmount();
  });

  it('ignores project-updated event if deletingProjectId matches', async () => {
    let contextValue: any;
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

    act(() => {
      contextValue.setProject({ id: 'p1', title: 'Test Project' });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    act(() => {
      contextValue.beginProjectDeletion('p1');
    });

    expect(eventCallback).toBeDefined();
    await act(async () => {
      await eventCallback({ payload: { projectId: 'p1' } });
    });

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
      contextValue.setProject({ id: 'p1', title: 'Test Project' } as any);
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    act(() => {
      void eventCallback({ payload: { projectId: 'p1' } });
    });

    expect(invoke).toHaveBeenCalledTimes(1);

    act(() => {
      contextValue.beginProjectDeletion('p1');
    });

    act(() => {
      contextValue.setProject(null);
      contextValue.finishProjectDeletion('p1');
    });

    await act(async () => {
      resolveGetProject({ id: 'p1', title: 'Resurrected' });
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    expect(contextValue.project).toBeNull();
  });

  it('verifies that token capture and validation function correctly', () => {
    let contextValue: any;
    render(
      <ProjectProvider>
        <TestComponent
          onContext={(ctx) => {
            contextValue = ctx;
          }}
        />
      </ProjectProvider>,
    );

    const token1 = contextValue.captureToken();
    expect(contextValue.validateToken(token1)).toBe(true);
  });

  it('updates selection atomically and invalidates tokens only on project changes', () => {
    let contextValue: any;
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
      contextValue.setProject({ id: 'p1', title: 'Test Project' });
    });
    const token1 = contextValue.captureToken();

    // Same ID set
    act(() => {
      contextValue.setProject({ id: 'p1', title: 'Test Project' });
    });
    expect(contextValue.validateToken(token1)).toBe(true);

    // Different ID set
    act(() => {
      contextValue.setProject({ id: 'p2', title: 'Test Project' });
    });
    expect(contextValue.validateToken(token1)).toBe(false);
  });

  it('binds operation tokens to the selected project context', () => {
    let contextValue: any;
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
      contextValue.setProject({ id: 'p1', title: 'Test Project' });
    });

    const token = contextValue.captureToken();
    expect(token.projectId).toBe('p1');
    expect(contextValue.validateToken(token)).toBe(true);

    const wrongProjectToken = {
      generation: token.generation,
      projectId: 'p2',
    };
    expect(contextValue.validateToken(wrongProjectToken)).toBe(false);
  });

  it('verifies an accepted beginProjectDeletion() invalidates the token synchronously before returning', () => {
    let contextValue: any;
    render(
      <ProjectProvider>
        <TestComponent
          onContext={(ctx) => {
            contextValue = ctx;
          }}
        />
      </ProjectProvider>,
    );

    const token1 = contextValue.captureToken();
    let result = false;
    act(() => {
      result = contextValue.beginProjectDeletion('p1');
    });

    expect(result).toBe(true);
    expect(contextValue.validateToken(token1)).toBe(false);
  });

  it('verifies that after beginProjectDeletion() followed by finishProjectDeletion(), the previously captured token remains invalid', () => {
    let contextValue: any;
    render(
      <ProjectProvider>
        <TestComponent
          onContext={(ctx) => {
            contextValue = ctx;
          }}
        />
      </ProjectProvider>,
    );

    const token1 = contextValue.captureToken();
    act(() => {
      contextValue.beginProjectDeletion('p1');
    });
    act(() => {
      contextValue.finishProjectDeletion('p1');
    });

    expect(contextValue.validateToken(token1)).toBe(false);
  });

  it('verifies that project-updated event fetch does not invalidate active operation tokens', async () => {
    let contextValue: any;
    let eventCallback: any;
    (listen as any).mockImplementation((event: string, cb: any) => {
      if (event === 'project-updated') eventCallback = cb;
      return Promise.resolve(() => {});
    });

    (invoke as Mock).mockResolvedValue({ id: 'p1', title: 'Updated' });

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
      contextValue.setProject({ id: 'p1', title: 'Test Project' });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    const token = contextValue.captureToken();

    await act(async () => {
      await eventCallback({ payload: { projectId: 'p1' } });
    });

    expect(contextValue.validateToken(token)).toBe(true);
  });

  it('verifies that two project-updated event fetches completing in reverse order apply only the latest one', async () => {
    let contextValue: any;
    let eventCallback: any;
    (listen as any).mockImplementation((event: string, cb: any) => {
      if (event === 'project-updated') eventCallback = cb;
      return Promise.resolve(() => {});
    });

    let resolve1: (val: any) => void = () => {};
    let resolve2: (val: any) => void = () => {};
    const promise1 = new Promise((resolve) => {
      resolve1 = resolve;
    });
    const promise2 = new Promise((resolve) => {
      resolve2 = resolve;
    });

    (invoke as Mock).mockReturnValueOnce(promise1).mockReturnValueOnce(promise2);

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
      contextValue.setProject({ id: 'p1', title: 'Test Project' });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // Fire first event (starts promise1)
    act(() => {
      void eventCallback({ payload: { projectId: 'p1' } });
    });

    // Fire second event (starts promise2)
    act(() => {
      void eventCallback({ payload: { projectId: 'p1' } });
    });

    // Resolve second fetch first
    await act(async () => {
      resolve2({ id: 'p1', title: 'Latest Title' });
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    expect(contextValue.project?.title).toBe('Latest Title');

    // Resolve first fetch second
    await act(async () => {
      resolve1({ id: 'p1', title: 'Stale Title' });
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // Should remain latest title
    expect(contextValue.project?.title).toBe('Latest Title');
  });
});
