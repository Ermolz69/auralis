import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { ExportPanel } from './ExportPanel';

const meta = {
  title: 'Widgets/ExportPanel/States',
  component: ExportPanel,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
} satisfies Meta<typeof ExportPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ExportUnavailable: Story = {
  render: () => (
    <div className="w-[560px]">
      <ExportPanel />
    </div>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const button = canvas.getByRole('button', { name: 'Export unavailable' });

    await expect(button).toBeDisabled();
    await expect(canvas.getByText(/not available in the current app contract/i)).toBeInTheDocument();
  },
};
