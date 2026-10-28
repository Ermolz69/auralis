import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import { ProjectListWorkflow, projectListBoundary } from './ProjectList.workflow.storyFixtures';

const meta = {
  title: 'Product/Features/Project List/Workflows',
  component: ProjectListWorkflow,
  tags: ['autodocs'],
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component:
          'Real project-list loading, deletion, busy protection and focus recovery with an isolated IPC boundary.',
      },
    },
  },
} satisfies Meta<typeof ProjectListWorkflow>;
export default meta;
type Story = StoryObj<typeof meta>;
let backend: ReturnType<typeof projectListBoundary>;

export const FetchRecovery: Story = {
  beforeEach: () => {
    backend = projectListBoundary(true);
    return backend.restore;
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole('alert')).toHaveTextContent('Project index unavailable');
    await userEvent.click(canvas.getByRole('button', { name: 'Retry', exact: true }));
    await expect(
      await canvas.findByRole('button', { name: /^Open Chapter one/ }),
    ).toBeInTheDocument();
    await expect(canvas.queryByRole('alert')).not.toBeInTheDocument();
  },
};

export const DeleteRecoveryAndFocus: Story = {
  beforeEach: () => {
    backend = projectListBoundary();
    return backend.restore;
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const document = within(canvasElement.ownerDocument.body);
    const remove = await canvas.findByRole('button', { name: 'Delete Chapter one', exact: true });
    remove.focus();
    await userEvent.keyboard('{Enter}');
    const firstDialog = await document.findByRole('dialog', { name: 'Delete Project' });
    await userEvent.click(within(firstDialog).getByRole('button', { name: 'Cancel', exact: true }));
    await waitFor(() => expect(remove).toHaveFocus());
    await expect(backend.deletions()).toBe(0);
    await userEvent.keyboard('{Enter}');
    const dialog = await document.findByRole('dialog', { name: 'Delete Project' });
    await userEvent.dblClick(within(dialog).getByRole('button', { name: 'Confirm Delete' }));
    await expect(backend.deletions()).toBe(1);
    await expect(remove).toBeDisabled();
    await expect(canvas.getByRole('button', { name: /^Open Chapter two/ })).toBeDisabled();
    backend.finishDeletion('Storage is temporarily unavailable');
    await expect(
      await document.findByText('Storage is temporarily unavailable'),
    ).toBeInTheDocument();
    await waitFor(() => expect(remove).toHaveFocus());
    await expect(remove).toBeEnabled();
    await userEvent.click(remove);
    const retryDialog = await document.findByRole('dialog', { name: 'Delete Project' });
    await userEvent.click(within(retryDialog).getByRole('button', { name: 'Confirm Delete' }));
    await expect(backend.deletions()).toBe(2);
    backend.finishDeletion();
    await waitFor(() =>
      expect(canvas.queryByRole('button', { name: /^Open Chapter one/ })).not.toBeInTheDocument(),
    );
    await expect(canvas.getByRole('button', { name: /^Open Chapter two/ })).toHaveFocus();
    await expect(
      canvas.getByRole('button', { name: 'Delete Chapter two', exact: true }),
    ).toBeEnabled();
  },
};
