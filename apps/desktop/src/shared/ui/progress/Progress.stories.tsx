import type { Meta, StoryObj } from '@storybook/react-vite';
import React from 'react';
import { Progress } from './Progress';

const meta = {
  title: 'Shared UI/Progress',
  component: Progress,
  parameters: {
    layout: 'padded',
  },
  tags: ['autodocs'],
  argTypes: {
    value: { control: { type: 'range', min: 0, max: 100 } },
    variant: { control: 'select', options: ['default', 'success', 'warning', 'danger'] },
    indeterminate: { control: 'boolean' },
  },
} satisfies Meta<typeof Progress>;

export default meta;
type Story = StoryObj<typeof meta>;

// 35 percent
export const Default: Story = {
  args: { value: 35 },
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
    </div>
  ),
};

// Indeterminate
export const Indeterminate: Story = {
  args: { indeterminate: true, variant: 'default' },
  render: (args) => (
    <div className="w-96">
      <Progress {...args} />
    </div>
  ),
};
