// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { cancelJob } from '@/entities/job';
import type { JobDto } from '@/entities/job';
import { CancelJobButton } from './CancelJobButton';

vi.mock('@/entities/job', () => ({
  cancelJob: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('CancelJobButton', () => {
  it('shows an accessible error when cancellation fails', async () => {
    vi.mocked(cancelJob).mockRejectedValueOnce({
      code: 'BUSY',
      message: 'Job cannot be cancelled right now',
    });

    render(<CancelJobButton jobId="job-1" />);

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect((await screen.findByRole('alert')).textContent).toContain(
      'Cancel failed: Job cannot be cancelled right now',
    );
    expect(cancelJob).toHaveBeenCalledWith('job-1');
  });

  it('disables the action and announces the cancelling transition', async () => {
    let resolveCancel: (value: JobDto) => void = () => undefined;
    vi.mocked(cancelJob).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveCancel = resolve;
        }),
    );

    render(<CancelJobButton jobId="job-1" />);

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Cancelling...' }).disabled).toBe(
      true,
    );
    expect(screen.getByRole('button', { name: 'Cancelling...' }).getAttribute('aria-busy')).toBe(
      'true',
    );

    resolveCancel({
      id: 'job-1',
      revision: 1,
      projectId: 'project-1',
      title: 'Subtitle import',
      status: 'cancelled',
      stage: null,
      progress: {
        percent: 10,
        message: 'Cancelled',
        currentStep: null,
        processedItems: null,
        totalItems: null,
      },
      error: null,
      createdAt: '2026-08-02T00:00:00.000Z',
      updatedAt: '2026-08-02T00:00:01.000Z',
    });

    await waitFor(() => {
      expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Cancel' }).disabled).toBe(
        false,
      );
    });
    expect(screen.getByRole('status').textContent).toContain('Cancellation requested.');
  });

  it('does not send duplicate cancellation while a request is pending', async () => {
    let resolveCancel: (value: JobDto) => void = () => undefined;
    vi.mocked(cancelJob).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveCancel = resolve;
        }),
    );

    render(<CancelJobButton jobId="job-1" />);

    const button = screen.getByRole('button', { name: 'Cancel' });
    fireEvent.click(button);
    fireEvent.click(button);

    expect(cancelJob).toHaveBeenCalledTimes(1);

    resolveCancel({
      id: 'job-1',
      revision: 1,
      projectId: 'project-1',
      title: 'Subtitle import',
      status: 'cancelled',
      stage: null,
      progress: {
        percent: 10,
        message: 'Cancelled',
        currentStep: null,
        processedItems: null,
        totalItems: null,
      },
      error: null,
      createdAt: '2026-08-02T00:00:00.000Z',
      updatedAt: '2026-08-02T00:00:01.000Z',
    });

    await waitFor(() => {
      expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Cancel' }).disabled).toBe(
        false,
      );
    });
  });
});
