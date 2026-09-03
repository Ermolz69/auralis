// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@/shared/api/tauri';
import { projectUpdated, youtubeImportsChanged } from '@/entities/project';
import { toast } from '@/shared/ui/toast';
import { PendingYoutubeImports } from './PendingYoutubeImports';

vi.mock('@/shared/api/tauri', () => ({ invoke: vi.fn() }));
vi.mock('@/shared/ui/toast', () => ({ toast: { success: vi.fn(), error: vi.fn() } }));
vi.mock('@/entities/project', async (original) => ({
  ...(await original<typeof import('@/entities/project')>()),
  projectUpdated: vi.fn(),
}));

const pending = { projectId: 'p-1', title: 'Interrupted video', state: 'Staged' as const };
const project = { id: 'p-1', title: 'Saved video' };
afterEach(cleanup);
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === 'list_pending_youtube_imports_cmd') return [pending] as never;
    if (command === 'resume_youtube_import_cmd') return project as never;
    return undefined as never;
  });
});

describe('pending YouTube imports', () => {
  it('lists persisted imports without automatically downloading', async () => {
    render(<PendingYoutubeImports onCompleted={vi.fn()} />);
    expect(await screen.findByText(pending.title)).toBeTruthy();
    expect(screen.getByText('Ready to save')).toBeTruthy();
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it('resumes only on request and publishes the committed project', async () => {
    const completed = vi.fn();
    render(<PendingYoutubeImports onCompleted={completed} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Resume' }));
    await waitFor(() => expect(completed).toHaveBeenCalledOnce());
    expect(invoke).toHaveBeenCalledWith('resume_youtube_import_cmd', { projectId: 'p-1' });
    expect(projectUpdated).toHaveBeenCalledWith(project);
  });

  it('retains the retry action after a failed resume', async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'resume_youtube_import_cmd') throw new Error('Network interrupted');
      return [pending] as never;
    });
    render(<PendingYoutubeImports onCompleted={vi.fn()} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Resume' }));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
    expect(projectUpdated).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Resume' }).hasAttribute('disabled')).toBe(false);
  });

  it('discards the pending import without deleting or publishing a project', async () => {
    render(<PendingYoutubeImports onCompleted={vi.fn()} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Discard' }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('discard_youtube_import_cmd', { projectId: 'p-1' }),
    );
    expect(projectUpdated).not.toHaveBeenCalled();
  });

  it('refreshes when a failed foreground import changes the journal', async () => {
    vi.mocked(invoke).mockResolvedValueOnce([]);
    render(<PendingYoutubeImports onCompleted={vi.fn()} />);
    await waitFor(() => expect(invoke).toHaveBeenCalledOnce());
    youtubeImportsChanged();
    expect(await screen.findByText(pending.title)).toBeTruthy();
  });

  it('shows a retryable listing error rather than hiding it', async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error('Database unavailable'));
    render(<PendingYoutubeImports onCompleted={vi.fn()} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Retry' }));
    expect(await screen.findByText(pending.title)).toBeTruthy();
  });
});
