import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, within } from 'storybook/test';
import { useRef } from 'react';
import type { Project } from '@/entities/project';
import { ProjectListRows } from './ProjectListRows';
import {
  ProjectListEmptyState,
  ProjectListErrorState,
  ProjectListLoadingState,
} from './ProjectListStates';

const projects: Project[] = [
  {
    id: 'youtube-project',
    title: 'https://www.youtube.com/watch?v=private-source',
    status: 'processing',
    source: { kind: 'youtubeUrl', url: 'https://www.youtube.com/watch?v=private-source' },
    metadata: null,
    createdAt: '2026-08-02T00:00:00.000Z',
    updatedAt: '2026-08-02T00:00:00.000Z',
  },
  {
    id: 'local-project',
    title: 'C:\\Users\\person\\Videos\\private-folder\\clip.mp4',
    status: 'completed',
    source: {
      kind: 'externalLocalFile',
      path: 'C:\\Users\\person\\Videos\\private-folder\\clip.mp4',
    },
    metadata: null,
    createdAt: '2026-08-02T00:00:00.000Z',
    updatedAt: '2026-08-02T00:00:00.000Z',
  },
  {
    id: 'failed-project',
    title: 'Recovered subtitle import',
    status: 'failed',
    source: { kind: 'managedLocalFile', artifactId: 'artifact-1', originalFilename: 'lecture.mov' },
    metadata: null,
    createdAt: '2026-08-02T00:00:00.000Z',
    updatedAt: '2026-08-02T00:00:00.000Z',
  },
];

const meta = {
  title: 'Features/ProjectList/States',
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  render: () => <ProjectListScenario projects={projects} />,
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const RecentProjects: Story = {
  render: () => <ProjectListScenario projects={projects} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(
      canvas.getByRole('button', {
        name: 'Open YouTube project. Status: Processing. Source: YouTube source (youtube.com)',
      }),
    ).toBeInTheDocument();
    await expect(canvas.getByText('YouTube source (youtube.com)')).toBeInTheDocument();
    await expect(canvas.getAllByText('clip.mp4')).toHaveLength(2);
    await expect(canvas.queryByText(/Users\\person/)).not.toBeInTheDocument();
  },
};

export const Loading: Story = {
  render: () => <ProjectListLoadingState />,
};

export const Empty: Story = {
  render: () => <ProjectListEmptyState />,
};

export const FetchError: Story = {
  args: {
    onRetry: fn(),
  },
  render: ({ onRetry }: { onRetry?: () => void }) => (
    <ProjectListErrorState
      error="Could not read the project index"
      onRetry={onRetry ?? (() => undefined)}
    />
  ),
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole('button', { name: 'Retry' }));
    await expect(args.onRetry).toHaveBeenCalled();
  },
};

function ProjectListScenario({ projects }: { projects: Project[] }) {
  const openButtonRefs = useRef(new Map<string, HTMLButtonElement>());
  const deleteButtonRefs = useRef(new Map<string, HTMLButtonElement>());

  return (
    <section className="w-[520px]" aria-labelledby="recent-projects-story-heading">
      <h1 id="recent-projects-story-heading" className="mb-3 text-lg font-semibold text-text">
        Recent projects
      </h1>
      <ProjectListRows
        projects={projects}
        deletingProjectId={null}
        openButtonRefs={openButtonRefs}
        deleteButtonRefs={deleteButtonRefs}
        onOpen={() => undefined}
        onDelete={() => undefined}
      />
    </section>
  );
}
