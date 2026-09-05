import type { Meta, StoryObj } from '@storybook/react-vite';
import { Button } from '../button';
import { StateView } from './StateView';

const meta = {
  title: 'Shared UI/StateView',
  component: StateView,
  args: {
    icon: 'Inbox',
    title: 'Nothing here yet',
    description: 'New items will appear in this area.',
    className: 'min-h-64',
  },
  tags: ['autodocs'],
} satisfies Meta<typeof StateView>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Empty: Story = {};

export const Error: Story = {
  args: {
    icon: 'CircleAlert',
    title: 'Could not load content',
    description: 'Try the request again.',
    tone: 'danger',
    role: 'alert',
    action: <Button variant="secondary">Retry</Button>,
  },
};
