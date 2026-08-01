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

    fireEvent.submit(screen.getByRole('button', { name: 'Add YouTube source' }).closest('form')!);

    expect(mockStartProject).toHaveBeenCalledTimes(1);
  });

  it('shows specific pending handoff copy', () => {
    mockHook({ isStarting: true });
    render(<PasteYoutubeLink />);

    expect(screen.getByRole('status').textContent).toContain('Creating YouTube project');
    expect(screen.getByText(/workspace will show subtitle import progress/i)).not.toBeNull();
  });

  it('links accessible errors to the input', () => {
    mockHook({ error: 'Could not create project' });
    render(<PasteYoutubeLink />);

    const input = screen.getByRole('textbox', { name: 'YouTube video link' });
    const alert = screen.getByRole('alert');

    expect(alert.textContent).toBe('Could not create project');
    expect(input.getAttribute('aria-invalid')).toBe('true');
    expect(input.getAttribute('aria-describedby')).toBe(alert.id);
  });
});
