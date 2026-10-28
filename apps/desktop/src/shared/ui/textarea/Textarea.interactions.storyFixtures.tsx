import { expect, userEvent, within } from 'storybook/test';

export async function playWithLabel({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const field = canvas.getByRole('textbox', { name: 'Description' });
  await userEvent.click(canvas.getByText('Description', { selector: 'label' }));
  await expect(field).toHaveFocus();
  await userEvent.type(field, 'First line{Enter}Second line');
  await expect(field).toHaveValue('First line\nSecond line');
  await expect(window.getComputedStyle(field).resize).toBe('vertical');
}

export async function playErrorState({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const field = canvas.getByRole('textbox', { name: 'Bio' });
  await expect(field).toHaveAttribute('aria-invalid', 'true');
  await expect(field).toHaveAttribute('aria-errormessage', canvas.getByRole('alert').id);
  await expect(field).toHaveAccessibleDescription(
    'Write a short public summary. Bio must be at least 100 characters.',
  );
}

export async function playDisabled({ canvasElement }: { canvasElement: HTMLElement }) {
  const field = within(canvasElement).getByRole('textbox', { name: 'Read-only notes' });
  await userEvent.type(field, 'replacement');
  await expect(field).toBeDisabled();
  await expect(field).toHaveValue('These notes cannot be edited anymore.');
}

export async function playNativeEditingAndValidation({
  canvasElement,
}: {
  canvasElement: HTMLElement;
}) {
  const canvas = within(canvasElement);
  const field = canvas.getByRole<HTMLTextAreaElement>('textbox', { name: 'Review notes' });
  await userEvent.click(canvas.getByRole('button', { name: 'Save notes' }));
  await expect(field.validity.valueMissing).toBe(true);
  await expect(field).toHaveAttribute('aria-invalid', 'true');
  await userEvent.type(field, 'First{Enter}Second123456789');
  await expect(field).toHaveValue('First\nSecond123456');
  await expect(canvas.getByRole('status', { name: 'Saved notes' })).toHaveTextContent(
    'No submission',
  );
  await expect(window.getComputedStyle(field).resize).toBe('none');
  await userEvent.click(canvas.getByRole('button', { name: 'Save notes' }));
  await expect(canvas.getByRole('status', { name: 'Saved notes' })).toHaveTextContent(
    'First Second123456',
  );
  await expect(field).not.toHaveAttribute('aria-invalid');
  await expect(field).toHaveAccessibleDescription('Use several lines.');
}

export async function playDisabledAndReadOnly({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const field = canvas.getByRole('textbox', { name: 'Read-only notes' });
  await userEvent.click(canvas.getByRole('button', { name: 'Before notes' }));
  await userEvent.tab();
  await expect(field).toHaveFocus();
  await userEvent.type(field, 'replacement');
  await expect(field).toHaveValue('Inspectable notes');
  await expect(canvas.getByRole('textbox', { name: 'Disabled notes' })).toBeDisabled();
  await userEvent.tab();
  await expect(canvas.getByRole('button', { name: 'After notes' })).toHaveFocus();
}
