import { expect, userEvent, within, waitFor } from 'storybook/test';

export async function playBasicDialog({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const trigger = canvas.getByRole('button', { name: 'Open Dialog' });

  await userEvent.click(trigger);
  const dialog = await canvas.findByRole('dialog', { name: 'Project details' });
  await waitFor(() => expect(dialog).toBeVisible());

  await expect(dialog).toHaveAccessibleDescription(
    'Review the source summary before continuing with the workspace.',
  );
  await expect(canvas.getByRole('button', { name: 'Continue' })).toBeInTheDocument();
  await userEvent.keyboard('{Escape}');
  await waitFor(() => expect(dialog).not.toBeVisible(), { timeout: 5000 });
  await expect(trigger).toHaveFocus();
  await userEvent.keyboard('{Enter}');
  await waitFor(() => expect(dialog).toBeVisible());
  await userEvent.click(
    canvas.getByText('Source metadata and current operation status stay readable.'),
  );
  await expect(dialog).toBeVisible();
  await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(dialog).not.toBeVisible());
  await expect(trigger).toHaveFocus();
  await userEvent.click(trigger);
  await waitFor(() => expect(dialog).toBeVisible());
  await userEvent.click(within(dialog).getByRole('button', { name: 'Close dialog' }));
  await waitFor(() => expect(dialog).not.toBeVisible());
  await expect(trigger).toHaveFocus();
}

export async function playWithForm({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const trigger = canvas.getByRole('button', { name: 'Create Project' });
  await userEvent.click(trigger);
  const dialog = await canvas.findByRole('dialog', { name: 'Create a new project' });
  const fields = within(dialog);
  const title = fields.getByRole<HTMLInputElement>('textbox', { name: 'Project Name' });
  await expect(title).toHaveFocus();
  await userEvent.click(fields.getByRole('button', { name: 'Create' }));
  await expect(title.validity.valueMissing).toBe(true);
  await expect(fields.getByRole('status', { name: 'Project submissions' })).toHaveTextContent(
    /^0$/,
  );
  await userEvent.type(title, 'Story project');
  const source = fields.getByRole<HTMLInputElement>('textbox', { name: 'YouTube URL' });
  await userEvent.type(source, 'invalid');
  await userEvent.click(fields.getByRole('button', { name: 'Create' }));
  await expect(source.validity.typeMismatch).toBe(true);
  await userEvent.clear(source);
  await userEvent.type(source, 'https://example.com/video');
  await userEvent.click(fields.getByRole('button', { name: 'Create' }));
  await expect(fields.getByRole('status', { name: 'Saved project' })).toHaveTextContent(
    'Story project · https://example.com/video',
  );
  await expect(fields.getByRole('status', { name: 'Project submissions' })).toHaveTextContent(
    /^1$/,
  );
  await userEvent.click(fields.getByRole('button', { name: 'Cancel' }));
  await expect(dialog).not.toBeVisible();
  await expect(trigger).toHaveFocus();
  await expect(canvas.getByRole('status', { name: 'Completed submissions' })).toHaveTextContent(
    /^1$/,
  );
}

export async function playOpenForAccessibility({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Review preferences' }));
  const dialog = await canvas.findByRole('dialog', { name: 'Review preferences' });
  await waitFor(() => expect(dialog).toBeVisible());
  await expect(dialog).toHaveAccessibleDescription('Choose readable labels before continuing.');
  await expect(within(dialog).getByRole('textbox', { name: 'Display name' })).toHaveFocus();
  for (let index = 0; index < 3; index++) {
    await userEvent.tab();
    await expect(dialog).toContainElement(document.activeElement as HTMLElement);
  }
  for (let index = 0; index < 3; index++) {
    await userEvent.tab({ shift: true });
    await expect(dialog).toContainElement(document.activeElement as HTMLElement);
  }
  await expect(within(dialog).getByRole('textbox', { name: 'Display name' })).toHaveFocus();
  await expect(dialog).toBeVisible();
}

export async function playPreventedActions({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Prevented trigger' }));
  await expect(canvas.queryByRole('dialog')).not.toBeInTheDocument();
  const trigger = canvas.getByRole('button', { name: 'Open editable dialog' });
  await userEvent.click(trigger);
  const dialog = await canvas.findByRole('dialog', { name: 'Editing choices' });
  await waitFor(() => expect(dialog).toBeVisible());
  await userEvent.click(within(dialog).getByRole('button', { name: 'Keep editing' }));
  await expect(dialog).toBeVisible();
  await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel editing' }));
  await expect(dialog).not.toBeVisible();
  await expect(trigger).toHaveFocus();
}
