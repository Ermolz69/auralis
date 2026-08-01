import type { Meta, StoryObj } from '@storybook/react-vite';
import type React from 'react';
import { expect, fn, userEvent, within } from 'storybook/test';
import { Button } from '@/shared/ui/button';
import { Input } from '@/shared/ui/input';
import { Page, PageContainer, PageContent } from '@/shared/ui/page-layout';
import { ProjectListEmptyState, ProjectListErrorState } from '@/features/project-list';

const meta = {
  title: 'Pages/Home/States',
  parameters: {
    layout: 'fullscreen',
  },
  tags: ['autodocs'],
  render: () => <HomeScenario />,
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const LocalFirstEmpty: Story = {
  render: () => <HomeScenario />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.getByRole('button', { name: 'Import local video' })).toBeInTheDocument();
    await expect(canvas.getByText('No projects yet')).toBeInTheDocument();
  },
};

export const ProjectListRecovery: Story = {
  args: {
    onRetry: fn(),
  },
  render: ({ onRetry }: { onRetry?: () => void }) => (
    <HomeScenario list={<ProjectListErrorState error="Storage index is unavailable" onRetry={onRetry ?? (() => undefined)} />} />
  ),
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole('button', { name: 'Retry' }));
    await expect(args.onRetry).toHaveBeenCalled();
  },
};

function HomeScenario({ list = <ProjectListEmptyState /> }: { list?: React.ReactNode }) {
  return (
    <Page className="flex flex-col items-center justify-center overflow-y-auto">
      <PageContainer size="sm" className="text-center justify-center items-center py-10">
        <PageContent className="items-center justify-center gap-7">
          <h1 className="text-5xl font-bold bg-gradient-to-r from-primary to-danger bg-clip-text text-transparent pb-2">
            Auralis
          </h1>
          <p className="text-muted text-xl">Import a local video and keep the work on your desktop.</p>
          <div className="mt-2 flex flex-col gap-4 w-full" aria-label="Create project">
            <Button type="button" size="lg" fullWidth>
              Import local video
            </Button>
            <div className="flex items-center gap-4 w-full">
              <hr className="flex-1 border-muted" />
              <span className="text-muted text-sm font-medium uppercase tracking-widest">
                or add a YouTube source
              </span>
              <hr className="flex-1 border-muted" />
            </div>
            <form className="flex flex-col gap-3" aria-label="Create YouTube project">
              <Input label="YouTube URL" placeholder="https://youtube.com/watch?v=..." />
              <Button type="submit" variant="secondary">
                Create subtitle project
              </Button>
            </form>
            <section className="w-full flex flex-col gap-3 mt-8" aria-labelledby="recent-projects-story-heading">
              <h2
                id="recent-projects-story-heading"
                className="text-sm font-semibold text-muted uppercase tracking-wider mb-2 text-left"
              >
                Recent Projects
              </h2>
              {list}
            </section>
          </div>
        </PageContent>
      </PageContainer>
    </Page>
  );
}
