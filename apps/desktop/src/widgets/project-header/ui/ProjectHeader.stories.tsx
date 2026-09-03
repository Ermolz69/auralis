import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { ProjectContext, type Project } from '@/entities/project';
import { NavigationProvider } from '@/shared/router';
import { ProjectHeader } from './ProjectHeader';

const youtubeProject: Project = {
  id: 'project-youtube',
  title: 'https://www.youtube.com/watch?v=private-source',
  status: 'ready_for_processing',
  source: { kind: 'youtubeUrl', url: 'https://www.youtube.com/watch?v=private-source' },
  metadata: null,
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:00.000Z',
};

const localProject: Project = {
  id: 'project-local',
  title: 'C:\\Users\\person\\Videos\\private-folder\\clip.mp4',
  status: 'source_imported',
  source: {
    kind: 'externalLocalFile',
    path: 'C:\\Users\\person\\Videos\\private-folder\\clip.mp4',
  },
  metadata: null,
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:00.000Z',
};

const meta = {
  title: 'Widgets/ProjectHeader/States',
  component: ProjectHeader,
  parameters: {
    layout: 'fullscreen',
  },
  tags: ['autodocs'],
  render: ({ project }: { project: Project }) => <ProjectHeaderStory project={project} />,
} satisfies Meta<{ project: Project }>;

export default meta;
type Story = StoryObj<typeof meta>;

export const YouTubeHandoff: Story = {
  args: {
    project: youtubeProject,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.getByRole('heading', { name: 'Источник видео' })).toBeInTheDocument();
    await expect(canvas.getByText('Ожидание')).toBeInTheDocument();
    await expect(canvas.getByTitle('YouTube source (youtube.com)')).toBeInTheDocument();
    await expect(canvas.queryByText('READY_FOR_PROCESSING')).not.toBeInTheDocument();
  },
};

export const LocalSourcePrivacy: Story = {
  args: {
    project: localProject,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.getByRole('heading', { name: 'Источник видео' })).toBeInTheDocument();
    await expect(canvas.getByTitle('clip.mp4')).toBeInTheDocument();
    await expect(canvas.queryByText(/Users\\person/)).not.toBeInTheDocument();
  },
};

function ProjectHeaderStory({ project }: { project: Project }) {
  return (
    <NavigationProvider>
      <ProjectContext.Provider
        value={{
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
        }}
      >
        <ProjectHeader />
      </ProjectContext.Provider>
    </NavigationProvider>
  );
}
