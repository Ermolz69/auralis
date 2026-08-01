// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { JobContext } from '@/entities/job';
import type { JobDto, JobStoreState } from '@/entities/job';
import { JobQueuePanel } from './JobQueuePanel';

const makeJob = (overrides: Partial<JobDto>): JobDto => ({
  id: 'job-1',
  revision: 1,
  projectId: 'project-1',
  title: 'Subtitle import',
  status: 'running',
  stage: 'importYoutubeSubtitles',
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

function renderPanel(state: Partial<JobStoreState> = {}) {
  const jobs = state.jobs ?? {};

  return render(
    <JobContext.Provider
      value={{
        phase: 'ready',
        scopeProjectId: 'p-1',
        jobs,
        buffer: [],
        pendingRefetch: false,
        generation: 0,
        ...state,
      }}
    >
      <JobQueuePanel />
    </JobContext.Provider>,
  );
}

afterEach(() => cleanup());

describe('JobQueuePanel', () => {
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

    expect(screen.getByText('Project: project-1')).not.toBeNull();
    expect(screen.getByText('Running - Import Youtube Subtitles')).not.toBeNull();
    expect(
      screen.getByRole('progressbar', { name: 'Subtitle import progress' }).getAttribute(
        'aria-valuenow',
      ),
    ).toBe('42');
    expect(screen.getByRole('button', { name: 'Cancel' })).not.toBeNull();
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
    expect(screen.getByText(/start a supported operation again/i)).not.toBeNull();
    expect(screen.queryByRole('progressbar')).toBeNull();
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
});
