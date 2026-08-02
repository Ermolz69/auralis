// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { useProjectContext } from '@/entities/project';
import type { ProjectContextType } from '@/entities/project';
import { useTranscript } from '@/entities/transcript';
import { JobContext } from '@/entities/job';
import type { JobDto, JobStoreState } from '@/entities/job';
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

const activeJob: JobDto = {
  id: 'job-1',
  revision: 1,
  projectId: 'project-1',
  title: 'Subtitle import',
  status: 'running',
  stage: 'importYoutubeSubtitles',
  progress: {
    percent: 42,
    message: 'Importing subtitles',
    currentStep: null,
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:01.000Z',
};

const jobState: JobStoreState = {
  phase: 'ready',
  scopeProjectId: 'project-1',
  jobs: {
    'job-1': activeJob,
  },
  buffer: [],
  pendingRefetch: false,
  generation: 1,
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

function renderTranscript(state: JobStoreState | null = null) {
  return render(
    <JobContext.Provider value={state}>
      <TranscriptEditor />
    </JobContext.Provider>,
  );
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('TranscriptEditor', () => {
  it('presents generated transcript text as read-only', () => {
    mockState();

    renderTranscript();

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

    renderTranscript();

    expect(screen.getByRole('alert').textContent).toContain('Transcript unavailable');
    fireEvent.click(screen.getByRole('button', { name: 'Retry loading' }));
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it('shows no-project transcript availability state', () => {
    mockState({
      projectContext: {
        ...defaultProjectContext,
        projectId: null,
        project: null,
      },
      transcriptState: {
        ...defaultTranscriptState,
        transcript: null,
      },
    });

    renderTranscript();

    expect(screen.getByText('No project selected')).not.toBeNull();
    expect(screen.getByText('Open a project to view transcript availability.')).not.toBeNull();
  });

  it('announces transcript fetch loading', () => {
    mockState({
      transcriptState: {
        ...defaultTranscriptState,
        transcript: null,
        isLoading: true,
      },
    });

    renderTranscript();

    expect(screen.getByRole('status').textContent).toContain('Loading transcript...');
  });

  it('links YouTube waiting state to the active job', () => {
    mockState({
      transcriptState: {
        ...defaultTranscriptState,
        transcript: null,
      },
    });

    renderTranscript(jobState);

    expect(screen.getByText('Waiting for subtitles')).not.toBeNull();
    expect(
      screen.getByText(/Linked operation: Running: Importing YouTube subtitles/i),
    ).not.toBeNull();
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

    renderTranscript();

    expect(screen.getByText(/Transcript unavailable/i)).not.toBeNull();
    expect(
      screen.getByText(/automatic transcription for local files is not supported/i),
    ).not.toBeNull();
  });
});
