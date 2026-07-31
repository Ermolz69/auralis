import type { Project } from '@/entities/project';
import { Button } from '@/shared/ui/button';
import {
  Dialog,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from '@/shared/ui/dialog';

type DeleteProjectDialogProps = {
  project: Project | null;
  isDeleting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
};

export const DeleteProjectDialog = ({
  project,
  isDeleting,
  onCancel,
  onConfirm,
}: DeleteProjectDialogProps) => (
  <Dialog open={!!project} onOpenChange={(open) => !open && onCancel()}>
    <form
      data-testid="delete-project-form"
      onSubmit={(event) => {
        event.preventDefault();
        onConfirm();
      }}
      className="contents"
    >
      <DialogHeader>
        <DialogTitle>Delete Project</DialogTitle>
        <DialogDescription>
          Are you sure you want to delete the project "{project?.title}"? This action cannot be
          undone.
        </DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <Button type="button" variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" variant="danger" loading={isDeleting}>
          Confirm Delete
        </Button>
      </DialogFooter>
      <DialogClose />
    </form>
  </Dialog>
);
