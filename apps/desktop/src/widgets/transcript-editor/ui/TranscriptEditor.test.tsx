// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { useProjectContext } from '@/entities/project';
import type { ProjectContextType } from '@/entities/project';
import { useTranscript } from '@/entities/transcript';
import { TranscriptEditor } from './TranscriptEditor';

vi.mock('@/entities/project', () => ({
  useProjectContext: vi.fn(),
}));

vi.mock('@/entities/transcript', () => ({
  useTranscript: vi.fn(),
}));

type TranscriptHookState = ReturnType<typeof useTranscript>;

const defaultProjectContext: ProjectContextType = {
  projectId: 'project-1',
  setProjectId: vi.fn(),
  project: {
    id: 'project-1',
    title: 'Demo project',
    status: 'completed',
    createdAt: '2026-08-02T00:00:00.000Z',
    updatedAt: '2026-08-02T00:00:01.000Z',
    source: {
      kind: 'youtubeUrl',
      url: 'https://youtube.com/watch?v=demo',
    },
    metadata: null,
  },
  setProject: vi.fn(),
  deletingProjectId: null,
  beginProjectDeletion: vi.fn(),
  finishProjectDeletion: vi.fn(),
  operationGeneration: 0,
  captureToken: vi.fn(() => ({ generation: 0, projectId: 'project-1' })),
  validateToken: vi.fn(() => true),
};

const defaultTranscriptState: TranscriptHookState = {
  transcript: {
    language: 'en',
    segments: [
      {
        id: 'segment-1',
        index: 0,
        startMs: 1000,
        endMs: 5000,
        sourceText: 'Hello from subtitles',
      },
    ],
  },
  isLoading: false,
  error: null,
  refetch: vi.fn(),
};

function mockState({
  projectContext = defaultProjectContext,
  transcriptState = defaultTranscriptState,
}: {
  projectContext?: ProjectContextType;
  transcriptState?: TranscriptHookState;
} = {}) {
  vi.mocked(useProjectContext).mockReturnValue(projectContext);
  vi.mocked(useTranscript).mockReturnValue(transcriptState);
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('TranscriptEditor', () => {
  it('presents generated transcript text as read-only', () => {
    mockState();

    render(<TranscriptEditor />);

    expect(screen.getByLabelText('Transcript viewer')).not.toBeNull();
    expect(screen.getByText('Read-only')).not.toBeNull();
    expect(screen.getByLabelText('Read-only transcript segments')).not.toBeNull();
    expect(screen.getByText('Hello from subtitles')).not.toBeNull();
    expect(screen.queryByRole('textbox')).toBeNull();
  });

  it('shows a real retry action when transcript loading fails', () => {
    const refetch = vi.fn();
    mockState({
      transcriptState: {
        ...defaultTranscriptState,
        transcript: null,
        error: 'Transcript unavailable',
        refetch,
      },
    });

    render(<TranscriptEditor />);

    expect(screen.getByRole('alert').textContent).toContain('Transcript unavailable');
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it('explains that local media transcription is unavailable', () => {
    const project = defaultProjectContext.project;
    if (!project) throw new Error('default project fixture is required');

    mockState({
      projectContext: {
        ...defaultProjectContext,
        project: {
          ...project,
          source: {
            kind: 'managedLocalFile',
            artifactId: 'artifact-1',
            originalFilename: 'clip.mp4',
          },
        },
      },
      transcriptState: {
        ...defaultTranscriptState,
        transcript: { language: 'en', segments: [] },
      },
    });

    render(<TranscriptEditor />);

    expect(screen.getByText('Transcript Unavailable')).not.toBeNull();
    expect(screen.getByText(/automatic transcription for local files is not supported/i)).not.toBeNull();
  });
});
