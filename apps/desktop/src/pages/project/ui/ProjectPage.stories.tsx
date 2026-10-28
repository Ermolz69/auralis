import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import { ProjectPage } from './ProjectPage';
import { ProjectWorkspaceStory, projectWorkspaceBoundary } from './ProjectPage.storyFixtures';

const meta = {
  title: 'Product/Pages/Project/Workspace',
  component: ProjectPage,
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component:
          'Project workspace navigation with real selection and operation-token ownership.',
      },
    },
  },
  tags: ['autodocs'],
  beforeEach: projectWorkspaceBoundary,
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
export const Medium1024: Story = { parameters: viewport('1024x720', 1024, 720) };
export const Small800: Story = { parameters: viewport('800x600', 800, 600) };
export const SubtitlesWide: Story = {
  parameters: viewport('1440x900', 1440, 900),
  render: () => <ProjectWorkspaceStory initialStep="subtitles" />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByRole('heading', { name: 'Текст и озвучка', level: 1 });
    await canvas.findByText(/Русский \(ru\)/, {}, { timeout: 10_000 });
  },
};
export const WorkspaceNavigation: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByLabelText('Video source configuration');
    await userEvent.click(canvas.getByRole('button', { name: /Текст, субтитры и озвучка/ }));
    const heading = await canvas.findByRole('heading', { name: 'Текст и озвучка', level: 1 });
    await waitFor(() => expect(heading).toHaveFocus());
    await expect(await canvas.findByText(/Русский \(ru\)/)).toBeInTheDocument();
    await expect(canvas.getByTitle('auralis')).toBeInTheDocument();
    await userEvent.click(canvas.getByRole('button', { name: /Источник видео/ }));
    const source = await canvas.findByRole('heading', { name: 'Источник видео', level: 1 });
    await expect(source).toHaveFocus();
    await expect(canvas.getByLabelText('Video source configuration')).toBeInTheDocument();
    await expect(canvas.getByTitle('auralis')).toBeInTheDocument();
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
