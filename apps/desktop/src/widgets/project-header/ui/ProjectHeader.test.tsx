// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { useProjectContext } from '@/entities/project';
import { useNavigation } from '@/shared/router';
import type { Project } from '@/entities/project';
import { ProjectHeader } from './ProjectHeader';

vi.mock('@/entities/project', () => ({
  useProjectContext: vi.fn(),
}));

vi.mock('@/shared/router', () => ({
  useNavigation: vi.fn(),
}));

vi.mock('../../../features/run-dubbing', () => ({
  RunDubbing: () => <button type="button">Run subtitles</button>,
}));

const project: Project = {
  id: 'project-1',
  title: 'https://www.youtube.com/watch?v=abc',
  status: 'ready_for_processing',
  source: {
    kind: 'youtubeUrl',
    url: 'https://www.youtube.com/watch?v=abc',
  },
  metadata: null,
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:01.000Z',
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('ProjectHeader', () => {
  it('renders safe project title, source label, and status label', () => {
    vi.mocked(useProjectContext).mockReturnValue({
      selection: project
        ? { status: 'open' as const, project: project }
        : { status: 'closed' as const },
      projectId: project.id,
      project,
      setProject: vi.fn(),
      deletingProjectId: null,
      beginProjectDeletion: vi.fn(),
      finishProjectDeletion: vi.fn(),
      operationGeneration: 0,
      captureToken: vi.fn(() => ({ generation: 0, projectId: project.id })),
      validateToken: vi.fn(() => true),
    });
    vi.mocked(useNavigation).mockReturnValue({
      currentView: 'project',
      setCurrentView: vi.fn(),
      pipelineStep: 'source',
      setPipelineStep: vi.fn(),
    });

    render(<ProjectHeader />);

    expect(screen.getByRole('heading', { name: 'Источник видео' })).not.toBeNull();
    expect(screen.getByText('Ожидание')).not.toBeNull();
    expect(screen.getByTitle('YouTube source (youtube.com)')).not.toBeNull();
    expect(screen.queryByText('READY_FOR_PROCESSING')).toBeNull();
    expect(screen.queryByText('https://www.youtube.com/watch?v=abc')).toBeNull();
  });
});
