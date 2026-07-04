import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
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
  title: 'UI Kit/Dialog',
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
        <DialogTitle>Edit profile</DialogTitle>
        <DialogDescription>Make changes to your profile here. Click save when you're done.</DialogDescription>
      </DialogHeader>
      <div className="py-4">
        <p className="text-sm">This is the dialog content body.</p>
      </div>
      <DialogFooter>
        <DialogCloseAction>
          <Button variant="ghost">Cancel</Button>
        </DialogCloseAction>
        <DialogCloseAction>
          <Button>Save changes</Button>
        </DialogCloseAction>
      </DialogFooter>
      <DialogClose />
    </Dialog>
  ),
};

export const ConfirmationDialog: Story = {
  render: () => (
    <Dialog trigger={<Button variant="secondary">Sign Out</Button>}>
      <DialogHeader>
        <DialogTitle>Sign Out</DialogTitle>
        <DialogDescription>Are you sure you want to sign out?</DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <DialogCloseAction>
          <Button variant="ghost">Cancel</Button>
        </DialogCloseAction>
        <DialogCloseAction>
          <Button variant="primary">Confirm</Button>
        </DialogCloseAction>
      </DialogFooter>
    </Dialog>
  ),
};

export const DangerConfirmation: Story = {
  render: () => (
    <Dialog trigger={<Button variant="danger">Delete Account</Button>}>
      <DialogHeader>
        <DialogTitle className="text-danger">Delete Account</DialogTitle>
        <DialogDescription>
          This action cannot be undone. This will permanently delete your account and remove your data from our servers.
        </DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <DialogCloseAction>
          <Button variant="ghost">Cancel</Button>
        </DialogCloseAction>
        <DialogCloseAction>
          <Button variant="danger">Delete Account</Button>
        </DialogCloseAction>
      </DialogFooter>
      <DialogClose />
    </Dialog>
  ),
};

export const LongContent: Story = {
  render: () => (
    <Dialog trigger={<Button>Terms of Service</Button>}>
      <DialogHeader>
        <DialogTitle>Terms of Service</DialogTitle>
      </DialogHeader>
      <div className="py-4 max-h-[300px] overflow-y-auto pr-2">
        {Array.from({ length: 10 }).map((_, i) => (
          <p key={i} className="mb-4 text-sm text-muted">
            Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam in odio felis. Suspendisse potenti. Vivamus
            vehicula velit in sagittis hendrerit. Phasellus mattis nisl nec magna vehicula, quis volutpat leo faucibus.
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
        <DialogDescription>Setup your new AI dubbing workspace.</DialogDescription>
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
