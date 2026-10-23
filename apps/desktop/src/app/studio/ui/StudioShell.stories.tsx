import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { StudioStory } from './StudioShell.storyFixtures';
const meta = {
  title: 'Product/Workspaces/Library and Tools',
  component: StudioStory,
  tags: ['autodocs'],
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component:
          'The library is a separate screen. Projects and standalone assets open workspaces with independent tool tabs. Fixtures are model-free; production interactions are covered by the library browser acceptance test.',
      },
    },
  },
} satisfies Meta<typeof StudioStory>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Library: Story = {
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole('heading', { name: 'Все материалы' })).toBeInTheDocument();
    await expect(canvas.queryByRole('tablist')).not.toBeInTheDocument();
  },
};
export const EmptyLibrary: Story = { args: { empty: true } };
export const ProjectWorkspace: Story = {
  args: { project: true, busy: true },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole('tablist')).toBeInTheDocument();
    await expect(canvas.getByRole('button', { name: 'Синтез речи' })).toBeInTheDocument();
    const resources = canvas.getByRole('complementary', { name: 'Материалы текущего проекта' });
    await expect(
      within(resources).getByRole('button', { name: /Вступление — русский текст/ }),
    ).toBeInTheDocument();
    const processes = canvas.getByRole('region', { name: 'Текущие процессы' });
    await expect(within(processes).getAllByRole('listitem')).toHaveLength(3);
    await expect(within(processes).queryByRole('button')).not.toBeInTheDocument();
  },
};
