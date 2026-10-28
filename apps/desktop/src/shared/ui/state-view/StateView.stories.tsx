import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, within } from 'storybook/test';
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
const retry = fn();

export const Empty: Story = {};

export const Error: Story = {
  args: {
    icon: 'CircleAlert',
    title: 'Could not load content',
    description: 'Try the request again.',
    tone: 'danger',
    role: 'alert',
    action: (
      <Button variant="secondary" onClick={retry}>
        Retry
      </Button>
    ),
  },
  play: async ({ canvasElement }) => {
    retry.mockClear();
    const canvas = within(canvasElement);
    const alert = canvas.getByRole('alert');
    await expect(alert).toHaveTextContent('Could not load content');
    await expect(alert).toHaveTextContent('Try the request again.');
    const action = within(alert).getByRole('button', { name: 'Retry' });
    action.focus();
    await expect(action).toHaveFocus();
    await userEvent.keyboard('{Enter}');
    await expect(retry).toHaveBeenCalledTimes(1);
    await expect(action).toHaveFocus();
  },
};

export const Loading: Story = {
  args: {
    title: 'Loading source details',
    description: 'Waiting for the project index.',
    role: 'status',
    live: 'polite',
    loading: true,
    density: 'compact',
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const status = canvas.getByRole('status');
    await expect(status).toHaveAttribute('aria-busy', 'true');
    await expect(status).toHaveAttribute('aria-live', 'polite');
    await expect(status).toHaveTextContent('Loading source details');
    await expect(canvas.queryByRole('button')).not.toBeInTheDocument();
  },
};
