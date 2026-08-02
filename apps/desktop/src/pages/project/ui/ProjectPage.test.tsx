// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { ProjectPage } from './ProjectPage';

vi.mock('@/entities/job', () => ({
  formatJobStatus: () => 'Running: Importing YouTube subtitles',
  getJobStatusTone: () => 'default',
  isActiveJobStatus: () => true,
  useJobContext: () => ({
    activeJobs: [
      {
        id: 'job-1',
        title: 'Subtitle import',
        status: 'running',
        progress: { percent: 42, message: 'Importing subtitles' },
      },
    ],
    phase: 'ready',
    pendingRefetch: false,
    scopeProjectId: 'project-1',
  }),
}));

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
  it('keeps main content first and hides horizontal page overflow', () => {
    render(<ProjectPage />);

    expect(screen.getByTestId('project-workspace').className).toContain('overflow-hidden');
    expect(
      screen
        .getByTestId('workspace-main')
        .compareDocumentPosition(screen.getByLabelText('Media panel controls')) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it('supports wide panel collapse with semantic toggles', () => {
    render(<ProjectPage />);

    const mediaToggle = screen.getByRole('button', { name: 'Hide Media' });
    expect(mediaToggle.getAttribute('aria-expanded')).toBe('true');

    fireEvent.click(mediaToggle);

    expect(screen.getByRole('button', { name: 'Show Media' }).getAttribute('aria-expanded')).toBe(
      'false',
    );
  });

  it('exposes secondary panels in compact mode with one explicit reveal control each', () => {
    render(<ProjectPage />);

    expect(screen.getByRole('button', { name: 'Open Media' })).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Open Jobs' })).not.toBeNull();
  });
});
