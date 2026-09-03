import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { ProjectContext, type Project } from '@/entities/project';
import { MediaPanel } from './MediaPanel';

const baseProject: Project = {
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
  title: 'Widgets/MediaPanel/States',
  component: MediaPanel,
  parameters: {
    layout: 'centered',
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
  },
};

export const AudioOnlyWarning: Story = {
  args: {
    project: audioOnlyProject,
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
