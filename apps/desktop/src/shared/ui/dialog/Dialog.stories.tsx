import type { Meta, StoryObj } from '@storybook/react-vite';
import React from 'react';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import {
  Dialog,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
  DialogCloseAction,
} from './Dialog';
import { Button } from '../button';
import { Input } from '../input';

const meta = {
  title: 'Shared UI/Dialog',
  component: Dialog,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
} satisfies Meta<typeof Dialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const BasicDialog: Story = {
  render: () => (
    <Dialog trigger={<Button>Open Dialog</Button>}>
      <DialogHeader>
        <DialogTitle>Project details</DialogTitle>
        <DialogDescription>
          Review the source summary before continuing with the workspace.
        </DialogDescription>
      </DialogHeader>
      <div className="py-4">
        <p className="text-sm">Source metadata and current operation status stay readable.</p>
      </div>
      <DialogFooter>
        <DialogCloseAction>
          <Button variant="ghost">Cancel</Button>
        </DialogCloseAction>
        <DialogCloseAction>
          <Button>Continue</Button>
        </DialogCloseAction>
      </DialogFooter>
      <DialogClose />
    </Dialog>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const trigger = canvas.getByRole('button', { name: 'Open Dialog' });

    await userEvent.click(trigger);
    const dialog = await canvas.findByRole('dialog', { name: 'Project details' });

    await expect(dialog).toHaveAccessibleDescription(
      'Review the source summary before continuing with the workspace.',
    );
    await expect(canvas.getByRole('button', { name: 'Continue' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(dialog).not.toBeVisible());
  },
};

export const ConfirmationDialog: Story = {
  render: () => (
    <Dialog trigger={<Button variant="secondary">Leave Project</Button>}>
      <DialogHeader>
        <DialogTitle>Leave Project</DialogTitle>
        <DialogDescription>
          The active backend job keeps running while you browse.
        </DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <DialogCloseAction>
          <Button variant="ghost">Cancel</Button>
        </DialogCloseAction>
        <DialogCloseAction>
          <Button variant="primary">Leave project</Button>
        </DialogCloseAction>
      </DialogFooter>
    </Dialog>
  ),
};

export const DangerConfirmation: Story = {
  render: () => (
    <Dialog trigger={<Button variant="danger">Delete Project</Button>}>
      <DialogHeader>
        <DialogTitle className="text-danger">Delete Project</DialogTitle>
        <DialogDescription>
          This removes the saved project from this desktop workspace after backend confirmation.
        </DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <DialogCloseAction>
          <Button variant="ghost">Cancel</Button>
        </DialogCloseAction>
        <DialogCloseAction>
          <Button variant="danger">Delete Project</Button>
        </DialogCloseAction>
      </DialogFooter>
      <DialogClose />
    </Dialog>
  ),
};

export const LongContent: Story = {
  render: () => (
    <Dialog trigger={<Button>View Job History</Button>}>
      <DialogHeader>
        <DialogTitle>Job History</DialogTitle>
      </DialogHeader>
      <div className="py-4 max-h-[300px] overflow-y-auto pr-2">
        {Array.from({ length: 10 }).map((_, i) => (
          <p key={i} className="mb-4 text-sm text-muted">
            Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam in odio felis.
            Suspendisse potenti. Vivamus vehicula velit in sagittis hendrerit. Phasellus mattis nisl
            nec magna vehicula, quis volutpat leo faucibus.
          </p>
        ))}
      </div>
      <DialogFooter>
        <DialogCloseAction>
          <Button>I Accept</Button>
        </DialogCloseAction>
      </DialogFooter>
      <DialogClose />
    </Dialog>
  ),
};

export const WithForm: Story = {
  render: () => (
    <Dialog trigger={<Button>Create Project</Button>}>
      <DialogHeader>
        <DialogTitle>Create a new project</DialogTitle>
        <DialogDescription>Add a supported source and open a project workspace.</DialogDescription>
      </DialogHeader>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          alert('Saved!');
        }}
        className="py-4 flex flex-col gap-4"
      >
        <Input label="Project Name" placeholder="e.g. My Awesome Video" />
        <Input label="YouTube URL" placeholder="https://youtube.com/..." />
        <DialogFooter className="mt-2">
          <DialogCloseAction>
            <Button type="button" variant="ghost">
              Cancel
            </Button>
          </DialogCloseAction>
          <Button type="submit">Create</Button>
        </DialogFooter>
      </form>
      <DialogClose />
    </Dialog>
  ),
};
