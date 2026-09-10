import type { Meta, StoryObj } from '@storybook/react-vite';
import { Button } from '../button';
import { icons } from '../icon/registry';
import { StateView } from './StateView';

const meta = {
  title: 'Design System/Components/StateView',
  component: StateView,
  parameters: {
    controls: {
      disable: false,
      include: ['icon', 'tone', 'density', 'loading'],
    },
    docs: {
      description: {
        component:
          'Standardized empty, error, and explanatory state with an optional recovery action.',
      },
    },
  },
  args: {
    icon: 'Inbox',
    title: 'Nothing here yet',
    description: 'New items will appear in this area.',
    className: 'min-h-64',
  },
  tags: ['autodocs'],
  argTypes: {
    icon: { control: 'select', options: Object.keys(icons) },
    tone: { control: 'select', options: ['neutral', 'danger'] },
    density: { control: 'select', options: ['default', 'compact'] },
    loading: { control: 'boolean' },
  },
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
