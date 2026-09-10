import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import {
  CONTROL_SAFETY_ICON_COUNT,
  ControlSafetyVisuals,
} from './ControlSafetyVisuals.storyFixtures';
import { ControlSafetyFields } from './ControlSafetyFields.storyFixtures';

function ControlSafetyMatrix() {
  return (
    <main className="space-y-8 text-text">
      <h1 className="text-xl font-semibold">Storybook control safety matrix</h1>
      <ControlSafetyVisuals />
      <ControlSafetyFields />
    </main>
  );
}

const meta = {
  title: 'Design System/Foundations/Control Safety Matrix',
  component: ControlSafetyMatrix,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'Browser-rendered boundary matrix for every control type that Auralis explicitly exposes in Storybook.',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof ControlSafetyMatrix>;

export default meta;
type Story = StoryObj<typeof meta>;

export const AllAllowedValues: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.getAllByTestId('safe-button')).toHaveLength(14);
    await expect(canvas.getAllByTestId('safe-badge')).toHaveLength(14);
    await expect(canvas.getAllByTestId('safe-card')).toHaveLength(4);
    await expect(canvas.getAllByTestId('safe-notice')).toHaveLength(4);
    await expect(canvas.getAllByTestId('safe-state-view')).toHaveLength(4);
    await expect(canvas.getAllByTestId('safe-icon')).toHaveLength(CONTROL_SAFETY_ICON_COUNT);
    await expect(canvas.getAllByTestId('safe-progress')).toHaveLength(13);
    await expect(canvas.getAllByTestId('safe-input')).toHaveLength(4);
    await expect(canvas.getAllByTestId('safe-textarea')).toHaveLength(4);
    await expect(canvas.getAllByTestId('safe-select')).toHaveLength(2);
  },
};
