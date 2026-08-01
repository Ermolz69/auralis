// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { ProjectPage } from './ProjectPage';

vi.mock('../../../widgets/project-header', () => ({
  ProjectHeader: () => <header>Project header</header>,
}));

vi.mock('../../../widgets/transcript-editor', () => ({
  TranscriptEditor: () => <section aria-label="Transcript">Transcript content</section>,
}));

vi.mock('../../../widgets/export-panel', () => ({
  ExportPanel: () => <section aria-label="Export">Export content</section>,
}));

vi.mock('../../../widgets/media-panel', () => ({
  MediaPanel: ({ className = '' }: { className?: string }) => (
    <aside aria-label="Media details" className={className}>
      Media details
    </aside>
  ),
}));

vi.mock('../../../widgets/job-queue-panel', () => ({
  JobQueuePanel: ({ className = '' }: { className?: string }) => (
    <aside aria-label="Job queue" className={className}>
      Jobs
    </aside>
  ),
}));

afterEach(() => cleanup());

describe('ProjectPage', () => {
  it('keeps a wide workspace and compact tabbed workspace without horizontal page overflow', () => {
    render(<ProjectPage />);

    expect(screen.getByTestId('project-workspace').className).toContain('overflow-hidden');
    expect(screen.getByTestId('workspace-wide').className).toContain('xl:grid');
    expect(screen.getByTestId('workspace-compact').className).toContain('xl:hidden');
  });

  it('exposes secondary media and jobs panels through compact tabs', () => {
    render(<ProjectPage />);

    expect(screen.getByRole('tab', { name: 'Media' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getAllByLabelText('Media details')).toHaveLength(2);

    fireEvent.click(screen.getByRole('tab', { name: 'Jobs' }));

    expect(screen.getByRole('tab', { name: 'Jobs' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getAllByLabelText('Job queue')).toHaveLength(2);
  });
});
