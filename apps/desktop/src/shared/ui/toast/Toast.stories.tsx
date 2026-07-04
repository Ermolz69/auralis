import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import { toast } from './toast';
import { Toaster } from './Toaster';
import { Button } from '../button';

const meta = {
  title: 'UI Kit/Toast',
  component: Toaster,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  decorators: [
    (Story) => (
      <div>
        <Story />
        <Toaster />
      </div>
    ),
  ],
} satisfies Meta<typeof Toaster>;

export default meta;
type Story = StoryObj<typeof meta>;

export const SuccessToast: Story = {
  render: () => (
    <Button onClick={() => toast.success('Video exported', { description: 'Your file has been saved to the Desktop.' })}>
      Show Success Toast
    </Button>
  ),
};

export const ErrorToast: Story = {
  render: () => (
    <Button variant="danger" onClick={() => toast.error('Export failed', { description: 'There was a problem connecting to the server. Please check your internet connection.' })}>
      Show Error Toast
    </Button>
  ),
};

export const WarningToast: Story = {
  render: () => (
    <Button variant="secondary" onClick={() => toast.warning('Low disk space', { description: 'You have less than 2GB of free space left.' })}>
      Show Warning Toast
    </Button>
  ),
};

export const InfoToast: Story = {
  render: () => (
    <Button variant="ghost" onClick={() => toast.default('Update available', { description: 'A new version of Auralis is ready to install.' })}>
      Show Info Toast
    </Button>
  ),
};

export const MultipleToasts: Story = {
  render: () => (
    <div className="flex gap-2">
      <Button onClick={() => toast.success('Task 1 completed')}>Task 1</Button>
      <Button onClick={() => toast.success('Task 2 completed')}>Task 2</Button>
      <Button variant="danger" onClick={() => toast.error('Task 3 failed')}>Task 3</Button>
    </div>
  ),
};

export const LongMessage: Story = {
  render: () => (
    <Button onClick={() => toast.default('Very long message', { description: 'This is a very long description that might wrap into multiple lines. Toasts should be able to handle long text without breaking the layout. The text will automatically wrap and might be clamped to save space.' })}>
      Show Long Message
    </Button>
  ),
};
