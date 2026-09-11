import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@/shared/api/tauri';
import { ArtifactRecovery } from './ArtifactRecovery';
vi.mock('@/shared/api/tauri', () => ({ invoke: vi.fn() }));
afterEach(cleanup);
beforeEach(() => {
  vi.mocked(invoke)
    .mockReset()
    .mockImplementation(
      async (command) => (command === 'list_artifact_recovery_cmd' ? ['project-1'] : null) as never,
    );
});
describe('artifact recovery', () => {
  it('never resumes automatically and deduplicates a pending manual retry', async () => {
    let acknowledge!: () => void;
    render(<ArtifactRecovery projects={[]} />);
    const retry = await screen.findByRole('button', { name: 'Retry file finalization' });
    expect(invoke).toHaveBeenCalledTimes(1);
    vi.mocked(invoke)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            acknowledge = () => resolve(null as never);
          }),
      )
      .mockResolvedValueOnce([] as never);
    fireEvent.click(retry);
    fireEvent.click(retry);
    expect(invoke).toHaveBeenCalledTimes(2);
    await act(async () => acknowledge());
    expect((await screen.findByRole('status')).textContent).toContain(
      'Existing downloads will be reused',
    );
    expect(screen.queryByRole('button', { name: 'Retry file finalization' })).toBeNull();
  });
  it('keeps a failed retry visible and ignores a late read after unmount', async () => {
    const first = render(<ArtifactRecovery projects={[]} />);
    const retry = await screen.findByRole('button', { name: 'Retry file finalization' });
    vi.mocked(invoke).mockRejectedValueOnce(new Error('unavailable'));
    fireEvent.click(retry);
    await screen.findByRole('alert');
    await waitFor(() => expect(retry.hasAttribute('disabled')).toBe(false));
    first.unmount();
    let resolve!: (ids: string[]) => void;
    vi.mocked(invoke).mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done as typeof resolve;
        }),
    );
    const next = render(<ArtifactRecovery projects={[]} />);
    next.unmount();
    await act(async () => resolve(['late']));
    expect(screen.queryByRole('region')).toBeNull();
  });
});
