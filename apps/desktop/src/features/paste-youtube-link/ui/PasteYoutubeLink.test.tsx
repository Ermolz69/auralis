// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { PasteYoutubeLink } from './PasteYoutubeLink';
import { usePasteYoutubeLink } from '../model/usePasteYoutubeLink';

vi.mock('../model/usePasteYoutubeLink', () => ({
  usePasteYoutubeLink: vi.fn(),
}));

const mockStartProject = vi.fn();
const mockSetUrl = vi.fn();

function mockHook(overrides: Partial<ReturnType<typeof usePasteYoutubeLink>> = {}) {
  vi.mocked(usePasteYoutubeLink).mockReturnValue({
    url: 'https://youtube.com/watch?v=123',
    setUrl: mockSetUrl,
    startProject: mockStartProject,
    isStarting: false,
    isBlockedByDeletion: false,
    error: null,
    ...overrides,
  });
}

describe('PasteYoutubeLink', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('submits semantically when Enter submits the form', () => {
    mockHook();
    render(<PasteYoutubeLink />);

    fireEvent.submit(screen.getByRole('form', { name: 'Add YouTube source' }));

    expect(mockStartProject).toHaveBeenCalledTimes(1);
  });

  it('shows specific pending handoff copy', () => {
    mockHook({ isStarting: true });
    render(<PasteYoutubeLink />);

    expect(screen.getByRole('button', { name: 'Add from YouTube' })).not.toBeNull();
    expect(screen.getByRole('status').textContent).toContain('Loading video metadata');
    expect(screen.getByText(/subtitles start on step 2/i)).not.toBeNull();
  });

  it('links accessible errors to the input', () => {
    mockHook({ error: 'Could not create project' });
    render(<PasteYoutubeLink />);

    const input = screen.getByRole('textbox', { name: 'YouTube URL' });
    const alert = screen.getByRole('alert');

    expect(alert.textContent).toBe('Could not create project');
    expect(input.getAttribute('aria-invalid')).toBe('true');
    expect(input.getAttribute('aria-describedby')).toContain(alert.id);
  });

  it('keeps focus on the input when submitting an empty URL', () => {
    mockHook({ url: '' });
    render(<PasteYoutubeLink />);

    fireEvent.submit(screen.getByRole('form', { name: 'Add YouTube source' }));

    const input = screen.getByRole('textbox', { name: 'YouTube URL' });
    expect(mockStartProject).not.toHaveBeenCalled();
    expect(screen.getByRole('alert').textContent).toBe(
      'Paste a YouTube URL before adding a source.',
    );
    expect(document.activeElement).toBe(input);
  });
});
