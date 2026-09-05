// @vitest-environment jsdom
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { WorkspaceSecondaryPanels } from './WorkspaceSecondaryPanels';

vi.mock('../../../widgets/media-panel', () => ({
  MediaPanel: ({ className }: { className?: string }) => (
    <div data-testid="media-panel" data-class-name={className} />
  ),
}));

vi.mock('../../../widgets/job-queue-panel', () => ({
  JobQueuePanel: ({ className }: { className?: string }) => (
    <div data-testid="job-queue-panel" data-class-name={className} />
  ),
}));

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = vi.fn(function showModal(this: HTMLDialogElement) {
    this.open = true;
  });
  HTMLDialogElement.prototype.close = vi.fn(function close(this: HTMLDialogElement) {
    this.open = false;
  });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('WorkspaceSecondaryPanels', () => {
  it('toggles the wide media panel and opens both compact dialogs', () => {
    render(<WorkspaceSecondaryPanels />);

    const hideButton = screen.getByRole('button', { name: 'Hide Media' });
    expect(hideButton.getAttribute('aria-expanded')).toBe('true');
    expect(hideButton.getAttribute('aria-controls')).toBe('workspace-media-panel');
    expect(document.getElementById('workspace-media-panel')).not.toBeNull();
    expect(screen.getAllByTestId('media-panel')).toHaveLength(2);

    fireEvent.click(hideButton);

    const showButton = screen.getByRole('button', { name: 'Show Media' });
    expect(showButton.getAttribute('aria-expanded')).toBe('false');
    expect(document.getElementById('workspace-media-panel')).toBeNull();
    expect(screen.getAllByTestId('media-panel')).toHaveLength(1);

    fireEvent.click(showButton);
    expect(document.getElementById('workspace-media-panel')).not.toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Open Media' }));
    const mediaDialog = screen.getByRole('dialog', { name: 'Media', hidden: true });
    expect(mediaDialog.hasAttribute('open')).toBe(true);
    expect(mediaDialog.getAttribute('aria-describedby')).not.toBeNull();
    expect(screen.getByText('Media details and stream information')).not.toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Open Jobs' }));
    const jobsDialog = screen.getByRole('dialog', { name: 'Jobs', hidden: true });
    expect(jobsDialog.hasAttribute('open')).toBe(true);
    expect(screen.getByText('Active and completed project jobs')).not.toBeNull();
    expect(screen.getByTestId('job-queue-panel')).not.toBeNull();
  });
});
