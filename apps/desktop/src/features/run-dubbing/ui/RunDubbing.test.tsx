// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, act, waitFor } from '@testing-library/react';
import { RunDubbing } from './RunDubbing';
import { ProjectContext, type Project, startProjectMockPipeline } from '@/entities/project';
import { toast } from '@/shared/ui/toast';

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

declare const require: any;

vi.mock('@/entities/project', () => {
  const React = require('react');
  const mockProjectContext = React.createContext(undefined);
  return {
    ProjectContext: mockProjectContext,
    startProjectMockPipeline: vi.fn(),
    useProjectContext: () => React.useContext(mockProjectContext),
  };
});

const mockProject: Project = {
  id: 'test-id',
  title: 'Test',
  status: 'ready_for_processing',
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
  source: { kind: 'remoteUrl', url: 'https://youtube.com/watch?v=123' },
  metadata: null,
};

describe('RunDubbing', () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  const createMockContext = (overrides = {}) => {
    const ctx = {
      projectId: mockProject.id,
      project: mockProject,
      setProjectId: vi.fn(),
      setProject: vi.fn(),
      deletingProjectId: null as string | null,
      beginProjectDeletion: vi.fn(),
      finishProjectDeletion: vi.fn(),
      operationGeneration: 1,
      captureToken: () => ({ generation: ctx.operationGeneration }),
      validateToken: (t: any) =>
        ctx.deletingProjectId === null && t.generation === ctx.operationGeneration,
      ...overrides,
    };
    return ctx;
  };

  it('is disabled during project deletion', () => {
    const ctx = createMockContext({ deletingProjectId: mockProject.id });
    render(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    expect(
      (screen.getByRole('button', { name: /run dubbing/i }) as HTMLButtonElement).disabled,
    ).toBe(true);
  });

  it('is enabled when project is eligible and not deleting', () => {
    const ctx = createMockContext();
    render(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    expect(
      (screen.getByRole('button', { name: /run dubbing/i }) as HTMLButtonElement).disabled,
    ).toBe(false);
  });

  it('is disabled when another project is deleting', () => {
    const ctx = createMockContext({ deletingProjectId: 'other-id' });
    const { rerender } = render(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    expect(
      (screen.getByRole('button', { name: /run dubbing/i }) as HTMLButtonElement).disabled,
    ).toBe(true);
    ctx.deletingProjectId = null;
    rerender(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    expect(
      (screen.getByRole('button', { name: /run dubbing/i }) as HTMLButtonElement).disabled,
    ).toBe(false);
  });

  it('verifies context switch resets loading for the new RunDubbing project', async () => {
    let resolveStart: (val: any) => void = () => {};
    const startPromise = new Promise((resolve) => {
      resolveStart = resolve;
    });
    vi.mocked(startProjectMockPipeline).mockReturnValue(startPromise as any);
    const ctx = createMockContext();
    const { rerender } = render(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    const btn = screen.getByRole('button', { name: /run dubbing/i });
    act(() => {
      btn.click();
    });
    expect((btn as HTMLButtonElement).disabled).toBe(true);
    ctx.projectId = 'new-id';
    ctx.project = { ...mockProject, id: 'new-id', status: 'ready_for_processing' };
    ctx.operationGeneration += 1;
    rerender(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    expect((btn as HTMLButtonElement).disabled).toBe(false);
    expect(screen.getByRole('button', { name: /run dubbing/i }).textContent).not.toBe(
      'Starting...',
    );
    await act(async () => {
      resolveStart({ project: { id: 'test-id', status: 'processing' } as any });
      await startPromise;
    });
    expect(ctx.setProject).not.toHaveBeenCalled();
  });

  it('fences out success when deletion occurs during RunDubbing', async () => {
    let resolveStart: (val: any) => void = () => {};
    const startPromise = new Promise((resolve) => {
      resolveStart = resolve;
    });
    vi.mocked(startProjectMockPipeline).mockReturnValue(startPromise as any);
    const ctx = createMockContext();
    const { rerender } = render(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    act(() => {
      screen.getByRole('button', { name: /run dubbing/i }).click();
    });
    ctx.operationGeneration += 1;
    ctx.deletingProjectId = 'test-id';
    rerender(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    await act(async () => {
      resolveStart({ project: { id: 'test-id', status: 'processing' } as any });
      await startPromise;
    });
    expect(ctx.setProject).not.toHaveBeenCalled();
  });

  it('stale RunDubbing error does not show toast or modify state', async () => {
    let rejectStart: (val: any) => void = () => {};
    const startPromise = new Promise((_, reject) => {
      rejectStart = reject;
    });
    startPromise.catch(() => {});
    vi.mocked(startProjectMockPipeline).mockReturnValue(startPromise as any);
    const ctx = createMockContext();
    const { rerender } = render(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    act(() => {
      screen.getByRole('button', { name: /run dubbing/i }).click();
    });
    ctx.operationGeneration += 1;
    rerender(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    await act(async () => {
      rejectStart(new Error('Pipeline failed'));
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    expect(toast.error).not.toHaveBeenCalled();
  });

  it('verifies that canonical selected project is not replaced by stale success', async () => {
    let resolveStart: (val: any) => void = () => {};
    const startPromise = new Promise((resolve) => {
      resolveStart = resolve;
    });
    vi.mocked(startProjectMockPipeline).mockReturnValue(startPromise as any);
    const ctx = createMockContext();
    const { rerender } = render(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    act(() => {
      screen.getByRole('button', { name: /run dubbing/i }).click();
    });
    ctx.projectId = 'new-id';
    ctx.project = { ...mockProject, id: 'new-id' };
    ctx.operationGeneration += 1;
    rerender(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );
    await act(async () => {
      resolveStart({ project: { id: 'test-id', status: 'processing' } as any });
      await startPromise;
    });
    expect(ctx.setProject).not.toHaveBeenCalled();
  });

  it('protects against raw leakage in run dubbing errors', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(startProjectMockPipeline).mockRejectedValue(
      new Error('C:\\Users\\secret\\video.mp4 token=SECRET') as any,
    );

    const ctx = createMockContext();
    render(
      <ProjectContext.Provider value={ctx}>
        <RunDubbing />
      </ProjectContext.Provider>,
    );

    await act(async () => {
      screen.getByRole('button', { name: /run dubbing/i }).click();
    });

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('An unexpected system error occurred');
    });

    consoleErrorSpy.mock.calls.forEach((call) => {
      const logStr = JSON.stringify(call);
      expect(logStr).not.toContain('secret');
      expect(logStr).not.toContain('SECRET');
      expect(logStr).not.toContain('token');
      expect(logStr).not.toContain('video.mp4');
    });

    consoleErrorSpy.mockRestore();
  });
});
