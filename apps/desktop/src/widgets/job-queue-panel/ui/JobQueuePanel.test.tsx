// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { JobContext, listJobHistoryPage } from '@/entities/job';
import type { JobDto, JobStoreState } from '@/entities/job';
import { ProjectContext, startProjectMockPipeline } from '@/entities/project';
import type { Project } from '@/entities/project';
import { JobQueuePanel } from './JobQueuePanel';

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

vi.mock('@/entities/job', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/entities/job')>()),
  listJobHistoryPage: vi.fn(),
}));

const makeJob = (overrides: Partial<JobDto>): JobDto => ({
  kind: 'dubbing',
  id: 'job-1',
  revision: 1,
  projectId: 'project-1',
  title: 'Subtitle import',
  status: 'running',
  stage: 'extractOrGenerateTranscript',
  progress: {
    percent: 42,
    message: 'Importing subtitles',
    currentStep: 'Import',
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:01.000Z',
  ...overrides,
});

const mockProject: Project = {
  revision: 1,
  id: 'project-1',
  title: 'https://youtube.com/watch?v=123',
  status: 'failed',
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:01.000Z',
  source: { kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=123' },
  metadata: null,
};

function renderPanel(state: Partial<JobStoreState> = {}, project: Project | null = mockProject) {
  const jobs = state.jobs ?? {};
  const projectContext = {
    selection: project
      ? { status: 'open' as const, project: project }
      : { status: 'closed' as const },
    projectId: project?.id ?? null,
    project,
    setProject: vi.fn(),
    deletingProjectId: null,
    beginProjectDeletion: vi.fn(),
    finishProjectDeletion: vi.fn(),
    operationGeneration: 1,
    captureToken: () => ({
      generation: 1,
      projectId: project?.id ?? null,
    }),
    validateToken: () => true,
  };

  return render(
    <ProjectContext.Provider value={projectContext}>
      <JobContext.Provider
        value={{
          phase: 'ready',
          jobs,
          buffer: [],
          pendingRefetch: false,
          generation: 0,
          ...state,
        }}
      >
        <JobQueuePanel />
      </JobContext.Provider>
    </ProjectContext.Provider>,
  );
}

afterEach(() => cleanup());
afterEach(() => vi.clearAllMocks());

beforeEach(() => {
  vi.mocked(listJobHistoryPage).mockImplementation(() => new Promise(() => undefined));
});

describe('JobQueuePanel', () => {
  it('shows all projects and unattached jobs when no project is selected', () => {
    renderPanel(
      {
        jobs: {
          first: makeJob({ id: 'first', title: 'First project job' }),
          second: makeJob({ id: 'second', projectId: 'project-2', title: 'Second project job' }),
          unattached: makeJob({ id: 'unattached', projectId: null, title: 'Unattached job' }),
          failed: makeJob({ id: 'failed', title: 'Failed job', status: 'failed' }),
        },
      },
      null,
    );
    expect(screen.getByRole('list', { name: 'Active operations' }).children).toHaveLength(3);
    expect(screen.getAllByRole('button', { name: 'Cancel' })).toHaveLength(3);
    expect(screen.getByRole('list', { name: 'Operation history' }).textContent).toContain(
      'Failed job',
    );
    expect(screen.queryByRole('button', { name: 'Retry subtitle import' })).toBeNull();
  });

  it('announces synchronization errors even without a selected project', () => {
    renderPanel({ phase: 'stale', pendingRefetch: true }, null);
    expect(screen.getByRole('alert').textContent).toContain('Job state may be outdated');
  });

  it('does not force a fixed desktop width on its root panel', () => {
    renderPanel();

    const panel = screen.getByLabelText('Job queue');

    expect(panel.className).toContain('min-w-0');
    expect(panel.className).not.toContain('w-96');
    expect(panel.className).not.toContain('shrink-0');
  });

  it('announces stale queue state while a refetch is pending', () => {
    renderPanel({
      phase: 'stale',
      pendingRefetch: true,
    });

    const alert = screen.getByRole('alert');

    expect(alert.textContent).toContain('Job state may be outdated');
    expect(alert.textContent).toContain('Avoid repeating actions until the queue updates.');
  });

  it('shows active job operation context and accessible progress', () => {
    renderPanel({
      jobs: {
        'job-1': makeJob({}),
      },
    });

    expect(screen.getByRole('heading', { name: 'Active operation' })).not.toBeNull();
    expect(screen.getByText('Project: YouTube project')).not.toBeNull();
    expect(screen.getByText('Running: Preparing transcript')).not.toBeNull();
    expect(
      screen
        .getByRole('progressbar', { name: 'Subtitle import progress' })
        .getAttribute('aria-valuenow'),
    ).toBe('42');
    expect(screen.getByRole('button', { name: 'Cancel' })).not.toBeNull();
  });

  it('shows cancelling as active until runtime confirms exit', () => {
    renderPanel({
      jobs: {
        'job-1': makeJob({
          status: 'cancelling',
          progress: {
            percent: 42,
            message: 'Waiting for runtime to stop',
            currentStep: 'Import',
            processedItems: null,
            totalItems: null,
          },
        }),
      },
    });

    expect(screen.getByRole('heading', { name: 'Active operation' })).not.toBeNull();
    expect(screen.getByText('Stopping')).not.toBeNull();
    expect(screen.getByText('Waiting for runtime to stop')).not.toBeNull();
    expect(screen.queryByRole('button', { name: 'Cancel' })).toBeNull();
    expect(
      screen
        .getByRole('progressbar', { name: 'Subtitle import progress' })
        .getAttribute('aria-valuenow'),
    ).toBeNull();
  });

  it('renders failed jobs with a safe next action instead of generic progress', () => {
    renderPanel({
      jobs: {
        'job-1': makeJob({
          status: 'failed',
          error: 'Subtitle import failed',
          progress: {
            percent: 70,
            message: 'Could not read subtitles',
            currentStep: null,
            processedItems: null,
            totalItems: null,
          },
        }),
      },
    });

    expect(screen.getByRole('alert').textContent).toContain('Subtitle import failed');
    expect(screen.getByText('Final state: Could not read subtitles')).not.toBeNull();
    expect(screen.getByText(/retry when this project has a supported source/i)).not.toBeNull();
    expect(screen.queryByRole('progressbar')).toBeNull();
  });

  it('offers retry only for the current project with a supported source', async () => {
    const responseProject = { ...mockProject, status: 'processing' as const };
    vi.mocked(startProjectMockPipeline).mockResolvedValueOnce({
      project: responseProject,
      job: makeJob({ id: 'retry-job', status: 'pending' }) as any,
    });

    renderPanel({
      jobs: {
        'job-1': makeJob({
          status: 'failed',
          error: 'Subtitle import failed',
        }),
      },
    });

    fireEvent.click(screen.getByRole('button', { name: 'Retry subtitle import' }));

    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Retrying...' }).disabled).toBe(
      true,
    );

    await waitFor(() => {
      expect(startProjectMockPipeline).toHaveBeenCalledTimes(1);
    });
    expect(startProjectMockPipeline).toHaveBeenCalledWith('project-1');
  });

  it('does not offer retry for local project sources', () => {
    renderPanel(
      {
        jobs: {
          'job-1': makeJob({
            status: 'failed',
            error: 'Subtitle import failed',
          }),
        },
      },
      {
        ...mockProject,
        source: {
          kind: 'managedLocalFile',
          artifactId: 'artifact-1',
          originalFilename: 'local-video.mp4',
        },
      },
    );

    expect(screen.queryByRole('button', { name: 'Retry subtitle import' })).toBeNull();
    expect(startProjectMockPipeline).not.toHaveBeenCalled();
  });

  it('distinguishes cancelled and completed terminal jobs', () => {
    renderPanel({
      jobs: {
        cancelled: makeJob({
          id: 'cancelled',
          title: 'Cancelled job',
          status: 'cancelled',
          progress: {
            percent: 10,
            message: 'User cancelled',
            currentStep: null,
            processedItems: null,
            totalItems: null,
          },
        }),
        completed: makeJob({
          id: 'completed',
          title: 'Completed job',
          status: 'completed',
          progress: {
            percent: 100,
            message: 'Transcript ready',
            currentStep: null,
            processedItems: null,
            totalItems: null,
          },
        }),
      },
    });

    expect(
      screen.getByText('Cancelled before completion. Start a new supported operation when ready.'),
    ).not.toBeNull();
    expect(screen.getByText('Completed successfully')).not.toBeNull();
    expect(screen.getByText('Transcript ready')).not.toBeNull();
  });

  it('loads global terminal history beyond the initial snapshot', async () => {
    const cursor = {
      createdAt: '2026-07-01T00:00:00.000Z',
      jobId: '00000000-0000-0000-0000-000000000100',
    };
    vi.mocked(listJobHistoryPage)
      .mockResolvedValueOnce({
        jobs: [
          makeJob({
            id: 'history-page-1',
            title: 'Page one job',
            status: 'completed',
            createdAt: '2026-07-02T00:00:00.000Z',
          }),
        ],
        nextCursor: cursor,
      })
      .mockResolvedValueOnce({
        jobs: [
          makeJob({
            id: 'history-page-2',
            title: 'Older than one hundred',
            status: 'failed',
            createdAt: '2026-06-01T00:00:00.000Z',
          }),
        ],
        nextCursor: null,
      });

    renderPanel({
      jobs: {
        current: makeJob({ id: 'current', title: 'Current job', status: 'completed' }),
      },
    });

    const loadMore = await screen.findByRole('button', { name: 'Load older jobs' });
    fireEvent.click(loadMore);

    expect(await screen.findByText('Older than one hundred')).not.toBeNull();
    expect(listJobHistoryPage).toHaveBeenNthCalledWith(1, null, 100);
    expect(listJobHistoryPage).toHaveBeenNthCalledWith(2, cursor, 100);
    expect(screen.queryByRole('button', { name: 'Load older jobs' })).toBeNull();
  });
});
