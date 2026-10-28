import type { Meta, StoryObj } from '@storybook/react-vite';
import React from 'react';
import { toast } from './toast';
import { Toaster } from './Toaster';
import { Button } from '../button';

import { ActionFeedback, SourceActionFeedback } from './Toast.storyFixtures';
import { resetStoryToasts } from './Toast.lifecycle.storyFixtures';
import {
  playSuccessToast,
  playErrorToast,
  playWarningToast,
  playInfoToast,
  playMultipleToasts,
  playLongMessage,
  playWithAction,
  playRepeatedAction,
} from './Toast.actions.storyFixtures';
import { playSwipeFeedback } from './Toast.gestures.storyFixtures';
import {
  playKeyboardFocusPausesDismissal,
  playHoverPausesDismissal,
} from './Toast.timing.storyFixtures';

const meta = {
  title: 'Design System/Components/Toast',
  component: Toaster,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component:
          'Transient, non-blocking feedback for completed operations, failures, warnings, and progress updates.',
      },
    },
  },
  tags: ['autodocs'],
  beforeEach: async () => {
    await resetStoryToasts();
    return resetStoryToasts;
  },
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
    <Button
      onClick={() =>
        toast.success('Video exported', { description: 'Your file has been saved to the Desktop.' })
      }
    >
      Show Success Toast
    </Button>
  ),
  play: playSuccessToast,
};

export const ErrorToast: Story = {
  render: () => (
    <Button
      variant="danger"
      onClick={() =>
        toast.error('Export failed', {
          description:
            'There was a problem connecting to the server. Please check your internet connection.',
        })
      }
    >
      Show Error Toast
    </Button>
  ),
  play: playErrorToast,
};

export const WarningToast: Story = {
  render: () => (
    <Button
      variant="secondary"
      onClick={() =>
        toast.warning('Low disk space', {
          description: 'You have less than 2GB of free space left.',
        })
      }
    >
      Show Warning Toast
    </Button>
  ),
  play: playWarningToast,
};

export const InfoToast: Story = {
  render: () => (
    <Button
      variant="ghost"
      onClick={() =>
        toast.default('Update available', {
          description: 'A new version of Auralis is ready to install.',
        })
      }
    >
      Show Info Toast
    </Button>
  ),
  play: playInfoToast,
};

export const MultipleToasts: Story = {
  render: () => (
    <div className="flex gap-2">
      <Button onClick={() => toast.success('Task 1 completed')}>Task 1</Button>
      <Button onClick={() => toast.success('Task 2 completed')}>Task 2</Button>
      <Button variant="danger" onClick={() => toast.error('Task 3 failed')}>
        Task 3
      </Button>
    </div>
  ),
  play: playMultipleToasts,
};

export const LongMessage: Story = {
  render: () => (
    <Button
      onClick={() =>
        toast.default('Very long message', {
          description:
            'This is a very long description that might wrap into multiple lines. Toasts should be able to handle long text without breaking the layout. The text will automatically wrap and might be clamped to save space.',
        })
      }
    >
      Show Long Message
    </Button>
  ),
  play: playLongMessage,
};

export const WithAction: Story = {
  render: () => <SourceActionFeedback />,
  play: playWithAction,
};

export const RepeatedAction: Story = {
  render: () => <ActionFeedback />,
  play: playRepeatedAction,
};

export const KeyboardFocusPausesDismissal: Story = {
  render: () => <ActionFeedback finite />,
  play: playKeyboardFocusPausesDismissal,
};

export const HoverPausesDismissal: Story = {
  render: () => <ActionFeedback finite />,
  play: playHoverPausesDismissal,
};

export const SwipeFeedback: Story = {
  render: () => <ActionFeedback />,
  play: playSwipeFeedback,
};
