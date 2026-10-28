import { useState } from 'react';
import { Button } from '../button';
import { Input } from '../input';
import {
  Dialog,
  DialogClose,
  DialogCloseAction,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './Dialog';

export function ProjectFormDialog() {
  const [saved, setSaved] = useState('No project saved');
  const [submissions, setSubmissions] = useState(0);
  return (
    <div>
      <Dialog trigger={<Button>Create Project</Button>}>
        <DialogHeader>
          <DialogTitle>Create a new project</DialogTitle>
          <DialogDescription>
            Add a supported source and open a project workspace.
          </DialogDescription>
        </DialogHeader>
        <form
          className="flex flex-col gap-4 py-4"
          onSubmit={(event) => {
            event.preventDefault();
            const data = new FormData(event.currentTarget);
            setSaved(`${data.get('title')} · ${data.get('source')}`);
            setSubmissions((value) => value + 1);
          }}
        >
          <Input
            label="Project Name"
            name="title"
            autoFocus
            required
            placeholder="e.g. My Awesome Video"
          />
          <Input
            label="YouTube URL"
            name="source"
            type="url"
            required
            placeholder="https://youtube.com/..."
          />
          <DialogFooter>
            <DialogCloseAction>
              <Button variant="ghost">Cancel</Button>
            </DialogCloseAction>
            <Button type="submit">Create</Button>
          </DialogFooter>
        </form>
        <p role="status" aria-label="Saved project">
          {saved}
        </p>
        <p role="status" aria-label="Project submissions">
          {submissions}
        </p>
        <DialogClose />
      </Dialog>
      <p role="status" aria-label="Completed submissions">
        {submissions}
      </p>
    </div>
  );
}

export function PreventedDialogActions() {
  return (
    <div className="flex gap-3">
      <Dialog
        trigger={<Button onClick={(event) => event.preventDefault()}>Prevented trigger</Button>}
      >
        <DialogTitle>Prevented opening</DialogTitle>
        <DialogClose />
      </Dialog>
      <Dialog trigger={<Button>Open editable dialog</Button>}>
        <DialogTitle>Editing choices</DialogTitle>
        <DialogDescription>Cancellation can be prevented by the owning action.</DialogDescription>
        <DialogFooter>
          <DialogCloseAction>
            <Button onClick={(event) => event.preventDefault()}>Keep editing</Button>
          </DialogCloseAction>
          <DialogCloseAction>
            <Button>Cancel editing</Button>
          </DialogCloseAction>
        </DialogFooter>
      </Dialog>
    </div>
  );
}

export function AccessibleOpenDialog() {
  return (
    <Dialog trigger={<Button>Review preferences</Button>}>
      <DialogHeader>
        <DialogTitle>Review preferences</DialogTitle>
        <DialogDescription>Choose readable labels before continuing.</DialogDescription>
      </DialogHeader>
      <div className="flex flex-col gap-4 py-4">
        <Input label="Display name" autoFocus defaultValue="Auralis" />
        <Input label="Reviewer notes" defaultValue="Keep these preferences" />
      </div>
      <DialogFooter>
        <DialogCloseAction>
          <Button>Done reviewing</Button>
        </DialogCloseAction>
      </DialogFooter>
      <DialogClose />
    </Dialog>
  );
}
