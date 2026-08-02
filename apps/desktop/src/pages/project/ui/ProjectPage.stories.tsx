import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { JobContext, type JobDto, type JobStoreState } from '@/entities/job';
import { ProjectContext, type Project, type ProjectContextType } from '@/entities/project';
import { NavigationProvider } from '@/shared/router';
import { ProjectPage } from './ProjectPage';

const project: Project = {
  id: 'workspace-story',
  title: 'Quarterly product walkthrough with a deliberately long title',
  status: 'processing',
  source: { kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=workspace' },
  metadata: {
    durationMs: 184000,
    width: 1920,
    height: 1080,
    fps: 29.97,
    videoCodec: 'h264',
    container: 'mp4',
    hasVideo: true,
    hasAudio: true,
    audioTracks: [],
    streams: [{ index: 0, codecType: 'video', codecName: 'h264', durationMs: 184000 }],
  },
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:01.000Z',
};

const runningJob: JobDto = {
  id: 'job-running',
  revision: 2,
  projectId: project.id,
  title: 'Subtitle import',
  status: 'running',
  stage: 'importYoutubeSubtitles',
  progress: {
    percent: 42,
    message: 'Importing subtitles',
    currentStep: 'subtitle-import',
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: project.createdAt,
  updatedAt: project.updatedAt,
};

const meta = {
  title: 'Pages/Project/Workspace',
  component: ProjectPage,
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
  render: () => <ProjectWorkspaceStory />,
} satisfies Meta<typeof ProjectPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Wide1280: Story = {
  parameters: viewport('1280x720', 1280, 720),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByLabelText('Current project work')).toBeInTheDocument();
    await expect(canvas.getByRole('button', { name: 'Hide Media' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
  },
};

export const Medium1024: Story = {
  parameters: viewport('1024x720', 1024, 720),
};

export const Small800: Story = { parameters: viewport('800x600', 800, 600) };

function viewport(name: string, width: number, height: number) {
  return {
    viewport: {
      defaultViewport: name,
      options: { [name]: { name, styles: { width: `${width}px`, height: `${height}px` } } },
    },
  };
}

function ProjectWorkspaceStory() {
  const projectContext: ProjectContextType = {
    projectId: project.id,
    project,
    setProjectId: () => undefined,
    setProject: () => undefined,
    deletingProjectId: null,
    beginProjectDeletion: () => false,
    finishProjectDeletion: () => undefined,
    operationGeneration: 0,
    captureToken: () => ({ generation: 0, projectId: project.id }),
    validateToken: () => true,
  };
  return (
    <NavigationProvider>
      <ProjectContext.Provider value={projectContext}>
        <JobContext.Provider value={createJobState()}>
          <ProjectPage />
        </JobContext.Provider>
      </ProjectContext.Provider>
    </NavigationProvider>
  );
}

function createJobState(): JobStoreState {
  return {
    phase: 'ready',
    scopeProjectId: project.id,
    jobs: { [runningJob.id]: runningJob },
    buffer: [],
    pendingRefetch: false,
    generation: 1,
  };
}
