import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import type { MediaSourceKind } from '@/entities/media';
import type { Transcript } from '@/entities/transcript';
import { TranscriptPanelView } from './TranscriptPanelView';

type TranscriptStoryState = {
  projectId: string | null;
  sourceKind: MediaSourceKind | null;
  transcript: Transcript | null;
  isLoading: boolean;
  error: string | null;
  activeJobStatus: string | null;
};

let storyTranscriptState: TranscriptStoryState = createTranscriptState({
  language: 'en',
  segments: [
    {
      id: 'segment-1',
      index: 0,
      startMs: 0,
      endMs: 3200,
      sourceText: 'Welcome to the product walkthrough.',
    },
    {
      id: 'segment-2',
      index: 1,
      startMs: 3200,
      endMs: 8200,
      sourceText:
        'This read-only transcript wraps long subtitle text without creating horizontal overflow.',
    },
  ],
});

const meta = {
  title: 'Widgets/TranscriptEditor/States',
  component: TranscriptPanelView,
  parameters: {
    layout: 'padded',
  },
  tags: ['autodocs'],
} satisfies Meta<typeof TranscriptPanelView>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ReadyReadOnlyTranscript: Story = {
  render: () => (
    <TranscriptStory transcriptState={createTranscriptState(storyTranscriptState.transcript)} />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.findByText('Read-only')).resolves.toBeInTheDocument();
    await expect(
      canvas.findByText('Welcome to the product walkthrough.'),
    ).resolves.toBeInTheDocument();
  },
};

export const WaitingForPipeline: Story = {
  render: () => (
    <TranscriptStory
      transcriptState={createTranscriptState(null, {
        projectId: 'youtube-waiting',
        sourceKind: 'youtubeUrl',
        activeJobStatus: 'Running: Importing YouTube subtitles',
      })}
    />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.findByText('Waiting for subtitles')).resolves.toBeInTheDocument();
    await expect(
      canvas.findByText(/Linked operation: Running: Importing YouTube subtitles/i),
    ).resolves.toBeInTheDocument();
  },
};

export const LocalMediaUnavailable: Story = {
  render: () => (
    <TranscriptStory
      transcriptState={createTranscriptState(null, {
        projectId: 'local-unavailable',
        sourceKind: 'managedLocalFile',
      })}
    />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.findByText(/Transcript unavailable/i)).resolves.toBeInTheDocument();
    await expect(
      canvas.findByText(/automatic transcription for local files is not supported/i),
    ).resolves.toBeInTheDocument();
  },
};

export const LoadingTranscript: Story = {
  render: () => (
    <TranscriptStory
      transcriptState={createTranscriptState(null, { projectId: 'youtube-ready', isLoading: true })}
    />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.findByRole('status')).resolves.toHaveTextContent('Loading transcript...');
  },
};

export const ErrorWithRetry: Story = {
  render: () => (
    <TranscriptStory
      transcriptState={createTranscriptState(null, { error: 'Transcript unavailable' })}
    />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.findByRole('alert')).resolves.toHaveTextContent('Transcript unavailable');
    await expect(
      canvas.findByRole('button', { name: 'Retry loading' }),
    ).resolves.toBeInTheDocument();
  },
};

export const NoProjectSelected: Story = {
  render: () => (
    <TranscriptStory transcriptState={createTranscriptState(null, { projectId: null })} />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.findByText('No project selected')).resolves.toBeInTheDocument();
  },
};

function TranscriptStory({ transcriptState }: { transcriptState: TranscriptStoryState }) {
  storyTranscriptState = transcriptState;

  return (
    <div className="h-[600px] flex">
      <TranscriptPanelView
        projectId={transcriptState.projectId}
        sourceKind={transcriptState.sourceKind}
        transcript={transcriptState.transcript}
        isLoading={transcriptState.isLoading}
        error={transcriptState.error}
        activeJobStatus={transcriptState.activeJobStatus}
        onRefetch={transcriptState.refetch}
      />
    </div>
  );
}

function createTranscriptState(
  transcript: Transcript | null,
  overrides: Partial<Omit<TranscriptStoryState, 'transcript' | 'refetch'>> = {},
) {
  return {
    projectId: 'projectId' in overrides ? overrides.projectId! : 'youtube-ready',
    sourceKind: 'sourceKind' in overrides ? overrides.sourceKind! : 'youtubeUrl',
    transcript,
    isLoading: overrides.isLoading ?? false,
    error: overrides.error ?? null,
    activeJobStatus: overrides.activeJobStatus ?? null,
    refetch: () => undefined,
  };
}
