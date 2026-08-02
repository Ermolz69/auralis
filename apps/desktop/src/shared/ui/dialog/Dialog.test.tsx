// @vitest-environment jsdom
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { Dialog, DialogCloseAction, DialogDescription, DialogTitle } from './Dialog';

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = vi.fn(function showModal(this: HTMLDialogElement) {
    this.open = true;
  });
  HTMLDialogElement.prototype.close = vi.fn(function close(this: HTMLDialogElement) {
    this.open = false;
  });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('Dialog', () => {
  it('keeps trigger and close actions semantic while linking title and description', () => {
    render(
      <Dialog trigger={<button type="button">Open dialog</button>}>
        <DialogTitle>Confirm delete</DialogTitle>
        <DialogDescription>This cannot be undone.</DialogDescription>
        <DialogCloseAction>
          <button type="button">Cancel</button>
        </DialogCloseAction>
      </Dialog>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Open dialog' }));

    const dialog = screen.getByRole('dialog', { hidden: true });
    const title = screen.getByRole('heading', { name: 'Confirm delete' });

    expect(dialog.getAttribute('aria-modal')).toBe('true');
    expect(dialog.getAttribute('aria-labelledby')).toBe(title.id);
    expect(dialog.getAttribute('aria-describedby')).not.toBe('');
    expect(screen.getByRole('button', { name: 'Cancel' })).not.toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(HTMLDialogElement.prototype.close).toHaveBeenCalled();
  });

  it('does not point aria-describedby at a missing description node', () => {
    render(
      <Dialog open>
        <DialogTitle>Preferences</DialogTitle>
      </Dialog>,
    );

    expect(screen.getByRole('dialog', { hidden: true }).hasAttribute('aria-describedby')).toBe(
      false,
    );
  });
});
