// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { ImportLocalMediaButton } from './ImportLocalMediaButton';
import { useImportLocalMedia } from '../model/useImportLocalMedia';

vi.mock('../model/useImportLocalMedia', () => ({
  useImportLocalMedia: vi.fn(),
}));

const baseState = {
  handleImport: vi.fn(),
  openDraftProject: vi.fn(),
  isImporting: false,
  isBlockedByDeletion: false,
  stage: 'idle',
  error: null,
  draftProject: null,
  sourceLabel: null,
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('ImportLocalMediaButton', () => {
  it('announces import stages with a safe source label and no fake percent', () => {
    vi.mocked(useImportLocalMedia).mockReturnValue({
      ...baseState,
      isImporting: true,
      stage: 'importing',
      sourceLabel: 'clip.mp4',
    } as any);

    render(<ImportLocalMediaButton />);

    expect(screen.getByRole('status').textContent).toBe('Importing into project: clip.mp4');
    expect(screen.queryByText(/%/)).toBeNull();
  });

  it('focuses draft-aware recovery summary after import failure', () => {
    vi.mocked(useImportLocalMedia).mockReturnValue({
      ...baseState,
      error: 'Import storage failed',
      draftProject: { id: 'p-new', title: 'clip.mp4' },
    } as any);

    render(<ImportLocalMediaButton />);

    const summary = screen.getByRole('alert');

    expect(document.activeElement).toBe(summary);
    expect(summary.textContent).toContain('A draft project was saved');
    expect(screen.getByRole('button', { name: 'Choose file again' })).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Open draft' })).not.toBeNull();
  });
});
