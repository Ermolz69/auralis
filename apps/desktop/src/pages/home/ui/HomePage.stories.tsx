import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, within } from 'storybook/test';
import { ProjectListErrorState, ProjectListLoadingState } from '@/features/project-list';
import { projects } from './HomePage.storyData';
import { HomeScenario, ProjectListRowsStory } from './HomePage.storyFixtures';

const meta = {
  title: 'Pages/Home/States',
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
  render: () => <HomeScenario />,
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const LocalFirstEmpty: Story = {
  render: () => <HomeScenario />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const localImport = canvas.getByRole('button', { name: 'Import local video' });

    await userEvent.tab();
    await expect(localImport).toHaveFocus();
    await expect(canvas.getByText('No projects yet')).toBeInTheDocument();
  },
};

export const ProjectListRecovery: Story = {
  args: { onRetry: fn() },
  render: ({ onRetry }: { onRetry?: () => void }) => (
    <HomeScenario
      list={
        <ProjectListErrorState
          error="Storage index is unavailable"
          onRetry={onRetry ?? (() => undefined)}
        />
      }
    />
  ),
  play: async ({ args, canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole('button', { name: 'Retry' }));
    await expect(args.onRetry).toHaveBeenCalled();
  },
};

export const LoadingRecentProjects: Story = {
  render: () => <HomeScenario list={<ProjectListLoadingState />} />,
};

export const ImportingLocalVideo: Story = {
  render: () => (
    <HomeScenario
      importStatus="Checking media: local-interview-with-a-long-safe-filename.mp4"
      importBusy
      list={<ProjectListRowsStory projects={projects} />}
    />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.getByRole('button', { name: 'Import local video' })).toHaveAttribute(
      'aria-busy',
      'true',
    );
    await expect(canvas.getByRole('status')).toHaveTextContent('Checking media');
  },
};

export const PopulatedRecentProjects: Story = {
  render: () => <HomeScenario list={<ProjectListRowsStory projects={projects} />} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.getByText('YouTube source (youtube.com)')).toBeInTheDocument();
    await expect(canvas.getAllByText('local-interview.mp4')).toHaveLength(2);
    await expect(canvas.queryByText(/Users\\person/)).not.toBeInTheDocument();
  },
};
