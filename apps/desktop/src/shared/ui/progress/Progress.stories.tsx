import type { Meta, StoryObj } from '@storybook/react-vite';
import React from 'react';
import { expect, within } from 'storybook/test';
import { Progress } from './Progress';

async function assertProgress(element: HTMLElement, value?: number) {
  await expect(element).toHaveAttribute('aria-valuemin', '0');
  await expect(element).toHaveAttribute('aria-valuemax', '100');
  await expect(element).toHaveAccessibleName();
  if (value === undefined) {
    await expect(element).not.toHaveAttribute('aria-valuenow');
    await expect(element).toHaveAttribute('aria-valuetext', 'Loading');
    await expect(element).toHaveAttribute('aria-busy', 'true');
  } else {
    await expect(element).toHaveAttribute('aria-valuenow', String(value));
    await expect(element).toHaveAttribute('aria-valuetext', `${value}%`);
    await expect(element).not.toHaveAttribute('aria-busy');
  }
}

const meta = {
  title: 'Design System/Components/Progress',
  component: Progress,
  parameters: {
    layout: 'padded',
    controls: {
      disable: false,
      include: ['value', 'variant', 'indeterminate'],
    },
    docs: {
      description: {
        component:
          'Determinate and indeterminate progress indicator with semantic status variants and accessible value metadata.',
      },
    },
  },
  tags: ['autodocs'],
  play: async ({ canvasElement, args }) => {
    await assertProgress(
      within(canvasElement).getByRole('progressbar'),
      args.indeterminate ? undefined : (args.value ?? 0),
    );
  },
  argTypes: {
    value: { control: { type: 'range', min: 0, max: 100, step: 1 } },
    variant: { control: 'select', options: ['default', 'success', 'warning', 'danger'] },
    indeterminate: { control: 'boolean' },
  },
} satisfies Meta<typeof Progress>;

export default meta;
type Story = StoryObj<typeof meta>;

// 35 percent
export const Default: Story = {
  args: { value: 35, label: 'Import progress' },
};

// 0 percent
export const ZeroPercent: Story = {
  args: { value: 0 },
};

// 100 percent
export const HundredPercent: Story = {
  args: { value: 100, variant: 'success' },
};

// Status variants
export const StatusVariants: Story = {
  render: () => (
    <div className="flex flex-col gap-4 w-96">
      <Progress value={20} variant="default" />
      <Progress value={45} variant="success" />
      <Progress value={65} variant="warning" />
      <Progress value={90} variant="danger" />
      <div className="flex items-center gap-2 text-sm text-success">
        <span className="h-2 w-2 rounded-full bg-success" aria-hidden="true" />
        <span>Completed successfully</span>
      </div>
    </div>
  ),
  play: async ({ canvasElement }) => {
    const indicators = within(canvasElement).getAllByRole('progressbar');
    await expect(indicators).toHaveLength(4);
    for (const [index, value] of [20, 45, 65, 90].entries()) {
      await assertProgress(indicators[index], value);
    }
  },
};

// Indeterminate
export const Indeterminate: Story = {
  args: { indeterminate: true, variant: 'default' },
  render: (args) => (
    <div className="w-96">
      <Progress {...args} label="Background operation progress" />
    </div>
  ),
};

export const ReducedMotionFallback: Story = {
  args: { indeterminate: true },
  render: () => (
    <div className="w-96 space-y-3">
      <Progress indeterminate label="Reduced motion fallback progress" />
      <p className="text-sm text-muted">
        The app stylesheet disables decorative animation when reduced motion is requested.
      </p>
    </div>
  ),
};
