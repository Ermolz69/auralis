import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { ProjectContext } from '@/entities/project';
import type { Project, ProjectContextType } from '@/entities/project';
import { TranscriptEditor } from './TranscriptEditor';

const youtubeProject: Project = {
  id: 'youtube-ready',
  title: 'Product walkthrough subtitles',
  status: 'completed',
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:00.000Z',
  metadata: null,
  source: {
    kind: 'youtubeUrl',
    url: 'https://youtube.com/watch?v=demo',
  },
};

const waitingProject: Project = {
  ...youtubeProject,
  id: 'youtube-waiting',
  title: 'Waiting for subtitles',
  status: 'processing',
};

const localProject: Project = {
  id: 'local-unavailable',
  title: 'Local media import',
  status: 'source_imported',
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:00.000Z',
  metadata: null,
  source: {
    kind: 'managedLocalFile',
    artifactId: 'artifact-local-test',
    originalFilename: 'file.mp4',
  },
};

const meta = {
  title: 'Widgets/TranscriptEditor/States',
  component: TranscriptEditor,
  parameters: {
    layout: 'padded',
  },
  tags: ['autodocs'],
} satisfies Meta<typeof TranscriptEditor>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ReadyReadOnlyTranscript: Story = {
  render: () => <TranscriptStory project={youtubeProject} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.findByText('Read-only')).resolves.toBeInTheDocument();
    await expect(canvas.findByText('Welcome to the product walkthrough.')).resolves.toBeInTheDocument();
  },
};

export const WaitingForPipeline: Story = {
  render: () => <TranscriptStory project={waitingProject} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.findByText('Waiting for transcript generation...')).resolves.toBeInTheDocument();
  },
};

export const LocalMediaUnavailable: Story = {
  render: () => <TranscriptStory project={localProject} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.findByText('Transcript Unavailable')).resolves.toBeInTheDocument();
    await expect(
      canvas.findByText(/automatic transcription for local files is not supported/i),
    ).resolves.toBeInTheDocument();
  },
};

function TranscriptStory({ project }: { project: Project }) {
  const context: ProjectContextType = {
    projectId: project.id,
    setProjectId: () => undefined,
    project,
    setProject: () => undefined,
    deletingProjectId: null,
    beginProjectDeletion: () => false,
    finishProjectDeletion: () => undefined,
    operationGeneration: 0,
    captureToken: () => ({ generation: 0, projectId: project.id }),
    validateToken: () => true,
  };

  return (
    <ProjectContext.Provider value={context}>
      <div className="h-[600px] flex">
        <TranscriptEditor />
      </div>
    </ProjectContext.Provider>
  );
}
