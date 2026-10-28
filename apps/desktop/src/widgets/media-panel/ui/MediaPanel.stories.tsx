import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, userEvent, within } from 'storybook/test';
import { ProjectContext, type Project } from '@/entities/project';
import { MediaPanel } from './MediaPanel';

const baseProject: Project = {
  revision: 1,
  id: 'project-media',
  title: 'Local review clip',
  status: 'source_imported',
  source: {
    kind: 'externalLocalFile',
    path: 'C:\\Users\\person\\Videos\\private-folder\\review.mov',
  },
  metadata: {
    durationMs: 184000,
    width: 1920,
    height: 1080,
    fps: 29.97,
    videoCodec: 'h264',
    audioCodec: 'aac',
    sampleRate: 48000,
    audioChannels: 2,
    container: 'mov',
    bitrate: 4200000,
    formatName: 'mov,mp4,m4a,3gp,3g2,mj2',
    hasVideo: true,
    hasAudio: true,
    audioTracks: [
      {
        streamIndex: 1,
        codec: 'aac',
        channels: 2,
        channelLayout: 'stereo',
        sampleRate: 48000,
        language: 'eng',
        title: 'Main audio',
        isDefault: true,
      },
    ],
    streams: [
      {
        index: 0,
        codecType: 'video',
        codecName: 'h264',
        codecLongName: 'H.264 / AVC',
        durationMs: 184000,
      },
      {
        index: 1,
        codecType: 'audio',
        codecName: 'aac',
        language: 'eng',
        durationMs: 184000,
      },
    ],
  },
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:00.000Z',
};

const audioOnlyProject: Project = {
  revision: 1,
  ...baseProject,
  id: 'project-audio-only',
  title: 'Audio only capture',
  metadata: baseProject.metadata
    ? {
        ...baseProject.metadata,
        hasVideo: false,
        width: undefined,
        height: undefined,
        videoCodec: undefined,
        streams: baseProject.metadata.streams.filter((stream) => stream.codecType === 'audio'),
      }
    : null,
};

const meta = {
  title: 'Product/Widgets/Media Panel/States',
  component: MediaPanel,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component:
          'Source-media summary for video, audio-only, sparse metadata, and unsupported stream combinations.',
      },
    },
  },
  tags: ['autodocs'],
  render: ({ project }: { project: Project }) => (
    <div className="h-[640px] w-80">
      <ProjectContext.Provider value={createProjectContext(project)}>
        <MediaPanel className="border border-muted" />
      </ProjectContext.Provider>
    </div>
  ),
} satisfies Meta<{ project: Project }>;

export default meta;
type Story = StoryObj<typeof meta>;

export const LocalMetadata: Story = {
  args: {
    project: baseProject,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.getByLabelText('Source: review.mov')).toBeInTheDocument();
    await expect(canvas.queryByText(/Users\\person/)).not.toBeInTheDocument();
    const summary = canvas.getByText('More details', { selector: 'summary' });
    await expect(summary.parentElement).not.toHaveAttribute('open');
    summary.focus();
    await expect(summary).toHaveFocus();
    await userEvent.click(summary);
    await expect(canvas.getByText('video Stream')).toBeVisible();
    await expect(canvas.getByText('audio Stream')).toBeVisible();
    await userEvent.click(summary);
    await expect(summary.parentElement).not.toHaveAttribute('open');
    await expect(canvas.getByText('video Stream')).not.toBeVisible();
    await expect(summary).toHaveFocus();
  },
};

export const AudioOnlyWarning: Story = {
  args: {
    project: audioOnlyProject,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('alert')).toHaveTextContent(
      'No video stream detected. Audio-only mode.',
    );
    await expect(canvas.queryByText('Resolution')).not.toBeInTheDocument();
    await expect(canvas.getByText('Track #0 (Main audio)')).toBeInTheDocument();
  },
};

export const MissingMetadata: Story = {
  args: { project: { ...baseProject, metadata: null } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText('No metadata available')).toBeInTheDocument();
    await expect(canvas.getByLabelText('Source: review.mov')).toBeInTheDocument();
    await expect(canvas.queryByText(/Users\\person/)).not.toBeInTheDocument();
    await expect(canvas.queryByText('More details')).not.toBeInTheDocument();
  },
};

export const MissingAudio: Story = {
  args: {
    project: {
      ...baseProject,
      metadata: {
        ...baseProject.metadata!,
        hasAudio: false,
        audioTracks: [],
        streams: baseProject.metadata!.streams.filter((stream) => stream.codecType === 'video'),
      },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('alert')).toHaveTextContent(
      'No audio tracks detected. Dubbing requires audio.',
    );
    await expect(canvas.queryByText('Main audio')).not.toBeInTheDocument();
    await expect(canvas.getByText('1920×1080')).toBeInTheDocument();
  },
};

function createProjectContext(project: Project) {
  return {
    selection: project
      ? { status: 'open' as const, project: project }
      : { status: 'closed' as const },
    projectId: project.id,
    project,
    setProject: () => undefined,
    deletingProjectId: null,
    beginProjectDeletion: () => false,
    finishProjectDeletion: () => undefined,
    operationGeneration: 0,
    captureToken: () => ({ generation: 0, projectId: project.id }),
    validateToken: () => true,
  };
}
