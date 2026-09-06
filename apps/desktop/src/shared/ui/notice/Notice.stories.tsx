import type { Meta, StoryObj } from '@storybook/react-vite';
import { Notice } from './Notice';

const meta = {
  title: 'Design System/Components/Notice',
  component: Notice,
  parameters: {
    docs: {
      description: {
        component:
          'Persistent inline feedback for informational, warning, error, and successful application states.',
      },
    },
  },
  args: {
    icon: 'Info',
    title: 'Operation continues in the background',
    children: 'You can safely navigate to another project.',
    tone: 'accent',
    role: 'status',
  },
  tags: ['autodocs'],
} satisfies Meta<typeof Notice>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Informational: Story = {};

export const Warning: Story = {
  args: {
    icon: 'CircleAlert',
    title: 'Data may be outdated',
    children: 'Wait for synchronization before repeating the action.',
    tone: 'warning',
    role: 'alert',
  },
};
