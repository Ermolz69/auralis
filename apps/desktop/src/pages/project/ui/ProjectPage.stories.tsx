import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { JobContext, type JobDto, type JobStoreState } from '@/entities/job';
import { ProjectContext, type Project, type ProjectContextType } from '@/entities/project';
import { NavigationProvider, useNavigation } from '@/shared/router';
import { AppShell } from '@/widgets/app-shell';
import { useEffect } from 'react';
import { ProjectPage } from './ProjectPage';

const project: Project = {
  id: 'workspace-story',
  title: 'auralis',
  status: 'processing',
  source: { kind: 'youtubeUrl', url: 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' },
  metadata: {
    durationMs: 2892000,
    width: 1920,
    height: 1080,
    fps: 30,
    videoCodec: 'h264',
    container: 'mp4',
    hasVideo: true,
    hasAudio: true,
    audioCodec: 'aac',
    sampleRate: 48000,
    audioChannels: 2,
    audioTracks: [
      {
        streamIndex: 1,
        codec: 'aac',
        channels: 2,
        sampleRate: 48000,
        language: 'en',
        isDefault: true,
      },
    ],
    streams: [
      { index: 0, codecType: 'video', codecName: 'h264', durationMs: 2892000 },
      { index: 1, codecType: 'audio', codecName: 'aac', language: 'en', durationMs: 2892000 },
    ],
  },
  createdAt: '2026-08-02T14:23:05.000Z',
  updatedAt: '2026-08-02T14:24:52.000Z',
};

const runningJob: JobDto = {
  kind: 'dubbing',
  id: 'job-running',
  revision: 2,
  projectId: project.id,
  title: 'Импорт субтитров',
  status: 'running',
  stage: 'extractOrGenerateTranscript',
  progress: {
    percent: 62,
    message: 'Импорт субтитров',
    currentStep: 'subtitle-import',
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: '2026-08-02T14:24:52.000Z',
  updatedAt: '2026-08-02T14:26:00.000Z',
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
    await expect(canvas.getByRole('heading', { name: 'Источник видео' })).toBeInTheDocument();
    await canvas.findByLabelText('Video source configuration', {}, { timeout: 10_000 });
  },
};

export const Medium1024: Story = {
  parameters: viewport('1024x720', 1024, 720),
};

export const Small800: Story = { parameters: viewport('800x600', 800, 600) };

export const SubtitlesWide: Story = {
  parameters: viewport('1440x900', 1440, 900),
  render: () => <ProjectWorkspaceStory initialStep="subtitles" />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('heading', { name: 'Субтитры' })).toBeInTheDocument();
    await canvas.findByText(/Русский \(ru\)/, {}, { timeout: 10_000 });
  },
};

function viewport(name: string, width: number, height: number) {
  return {
    viewport: {
      defaultViewport: name,
      options: { [name]: { name, styles: { width: `${width}px`, height: `${height}px` } } },
    },
  };
}

function ProjectWorkspaceStory({
  initialStep = 'source',
}: {
  initialStep?: 'source' | 'subtitles';
}) {
  const projectContext: ProjectContextType = {
    selection: { status: 'open', project },
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
  return (
    <NavigationProvider>
      <InitialProjectView step={initialStep} />
      <ProjectContext.Provider value={projectContext}>
        <JobContext.Provider value={createJobState()}>
          <AppShell>
            <ProjectPage />
          </AppShell>
        </JobContext.Provider>
      </ProjectContext.Provider>
    </NavigationProvider>
  );
}

function InitialProjectView({ step }: { step: 'source' | 'subtitles' }) {
  const { setCurrentView, setPipelineStep } = useNavigation();

  useEffect(() => {
    setPipelineStep(step);
    setCurrentView('project');
  }, [setCurrentView, setPipelineStep, step]);

  return null;
}

function createJobState(): JobStoreState {
  return {
    phase: 'ready',
    jobs: { [runningJob.id]: runningJob },
    buffer: [],
    pendingRefetch: false,
    generation: 1,
  };
}
