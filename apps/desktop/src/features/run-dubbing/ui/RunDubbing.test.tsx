// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import { RunDubbing } from './RunDubbing';
import { ProjectContext, type Project } from '@/entities/project';

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
  });

  it('is disabled during project deletion', () => {
    render(
      <ProjectContext.Provider
        value={{
          projectId: mockProject.id,
          project: mockProject,
          setProjectId: vi.fn(),
          setProject: vi.fn(),
          deletingProjectId: mockProject.id,
          beginProjectDeletion: vi.fn(),
          finishProjectDeletion: vi.fn(),
        }}
      >
        <RunDubbing />
      </ProjectContext.Provider>,
    );

    const btn = screen.getByRole('button', { name: /run dubbing/i });
    expect((btn as HTMLButtonElement).disabled).toBe(true);
  });

  it('is enabled when project is eligible and not deleting', () => {
    render(
      <ProjectContext.Provider
        value={{
          projectId: mockProject.id,
          project: mockProject,
          setProjectId: vi.fn(),
          setProject: vi.fn(),
          deletingProjectId: null,
          beginProjectDeletion: vi.fn(),
          finishProjectDeletion: vi.fn(),
        }}
      >
        <RunDubbing />
      </ProjectContext.Provider>,
    );

    const btn = screen.getByRole('button', { name: /run dubbing/i });
    expect((btn as HTMLButtonElement).disabled).toBe(false);
  });

  it('is disabled when another project is being deleted and re-enables after it finishes', () => {
    const { rerender } = render(
      <ProjectContext.Provider
        value={{
          projectId: mockProject.id,
          project: mockProject,
          setProjectId: vi.fn(),
          setProject: vi.fn(),
          deletingProjectId: 'other-id', // other project is being deleted
          beginProjectDeletion: vi.fn(),
          finishProjectDeletion: vi.fn(),
        }}
      >
        <RunDubbing />
      </ProjectContext.Provider>,
    );

    const btn = screen.getByRole('button', { name: /run dubbing/i });
    expect((btn as HTMLButtonElement).disabled).toBe(true);

    // Re-render with deletingProjectId cleared (deletion finished)
    rerender(
      <ProjectContext.Provider
        value={{
          projectId: mockProject.id,
          project: mockProject,
          setProjectId: vi.fn(),
          setProject: vi.fn(),
          deletingProjectId: null, // deletion finished
          beginProjectDeletion: vi.fn(),
          finishProjectDeletion: vi.fn(),
        }}
      >
        <RunDubbing />
      </ProjectContext.Provider>,
    );

    expect((btn as HTMLButtonElement).disabled).toBe(false);
  });
});
