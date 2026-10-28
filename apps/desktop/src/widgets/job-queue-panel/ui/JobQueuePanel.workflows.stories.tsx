import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import {
  QueueWorkflow,
  failedJob,
  finishedJob,
  queueBoundary,
  queueJob,
} from './JobQueuePanel.workflow.storyFixtures';

let backend: ReturnType<typeof queueBoundary>;
const meta = {
  title: 'Product/Widgets/Job Queue Panel/Workflows',
  component: QueueWorkflow,
  tags: ['autodocs'],
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component:
          'Queue cancellation, history paging and stale project retries using real operation guards and IPC fixtures.',
      },
    },
  },
  beforeEach: () => {
    backend = queueBoundary();
    return backend.restore;
  },
} satisfies Meta<typeof QueueWorkflow>;
export default meta;
type Story = StoryObj<typeof meta>;

export const CancellationRecovery: Story = {
  args: { jobs: [queueJob] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const cancel = await canvas.findByRole('button', { name: 'Cancel', exact: true });
    await userEvent.dblClick(cancel);
    await expect(backend.calls().cancel).toBe(1);
    await expect(canvas.getByRole('button', { name: 'Cancelling...' })).toBeDisabled();
    backend.finishCancel('Runtime is temporarily busy');
    await expect(await canvas.findByRole('alert')).toHaveTextContent(
      'Cancel failed: Runtime is temporarily busy',
    );
    await expect(cancel).toHaveAccessibleDescription('Cancel failed: Runtime is temporarily busy');
    await userEvent.click(cancel);
    await expect(backend.calls().cancel).toBe(2);
    backend.finishCancel();
    await expect(
      await canvas.findByText('Runtime stopped and cancellation confirmed.'),
    ).toBeInTheDocument();
    await expect(canvas.queryByRole('alert')).not.toBeInTheDocument();
  },
};

export const HistoryRecoveryAndPaging: Story = {
  beforeEach: () => {
    backend = queueBoundary({ historyFailure: true });
    return backend.restore;
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole('alert')).toHaveTextContent('History storage unavailable');
    await userEvent.click(canvas.getByRole('button', { name: 'Retry', exact: true }));
    await expect(await canvas.findByText('Earlier completed import')).toBeInTheDocument();
    await userEvent.dblClick(canvas.getByRole('button', { name: 'Load older jobs' }));
    await expect(canvas.getByRole('button', { name: 'Loading history...' })).toBeDisabled();
    await expect(backend.calls().history).toBe(3);
    backend.finishHistory({
      jobs: [
        { ...finishedJob, revision: 2, title: 'Updated earlier import' },
        {
          ...finishedJob,
          id: 'older-job',
          title: 'Older imported subtitles',
          createdAt: '2026-10-22T10:00:00Z',
        },
      ],
      nextCursor: null,
    });
    await expect(await canvas.findByText('Updated earlier import')).toBeInTheDocument();
    await expect(canvas.getAllByRole('listitem')).toHaveLength(2);
    await expect(canvas.queryByText('Earlier completed import')).not.toBeInTheDocument();
    await expect(canvas.queryByRole('button', { name: 'Load older jobs' })).not.toBeInTheDocument();
  },
};

export const RetryIgnoresPreviousProject: Story = {
  args: { jobs: [failedJob], switchProject: true },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const retry = await canvas.findByRole('button', { name: 'Retry subtitle import' });
    await userEvent.dblClick(retry);
    await expect(canvas.getByRole('button', { name: 'Retrying...' })).toBeDisabled();
    await expect(backend.calls().retry).toBe(1);
    await userEvent.click(canvas.getByRole('button', { name: 'Select Chapter two' }));
    backend.finishRetry();
    await waitFor(() =>
      expect(canvas.queryByRole('button', { name: 'Retrying...' })).not.toBeInTheDocument(),
    );
    await expect(canvas.getByText('Selected project: Chapter two')).toBeInTheDocument();
    await expect(
      canvas.queryByRole('button', { name: 'Retry subtitle import' }),
    ).not.toBeInTheDocument();
  },
};
