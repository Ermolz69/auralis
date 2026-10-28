import { expect, userEvent, within } from 'storybook/test';

export async function playWithLabel({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const input = canvas.getByRole('textbox', { name: 'Username' });
  await userEvent.click(canvas.getByText('Username', { selector: 'label' }));
  await expect(input).toHaveFocus();
  await userEvent.type(input, 'Auralis');
  await expect(input).toHaveValue('Auralis');
  await userEvent.clear(input);
  await expect(input).toHaveValue('');
}

export async function playWithHelperText({ canvasElement }: { canvasElement: HTMLElement }) {
  await expect(
    within(canvasElement).getByRole('textbox', { name: 'Email' }),
  ).toHaveAccessibleDescription("We'll never share your email.");
}

export async function playErrorState({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const input = canvas.getByRole('textbox', { name: 'Password' });
  await expect(input).toHaveAttribute('aria-invalid', 'true');
  await expect(input).toHaveAttribute('aria-errormessage', canvas.getByRole('alert').id);
  await expect(input).toHaveAccessibleDescription(
    'Use at least 8 characters. Password is required.',
  );
}

export async function playDisabled({ canvasElement }: { canvasElement: HTMLElement }) {
  const input = within(canvasElement).getByRole('textbox', { name: 'API Key' });
  await userEvent.type(input, 'replacement');
  await expect(input).toBeDisabled();
  await expect(input).toHaveValue('sk-1234567890');
}

export async function playNativeValidationAndRecovery({
  canvasElement,
}: {
  canvasElement: HTMLElement;
}) {
  const canvas = within(canvasElement);
  const input = canvas.getByRole<HTMLInputElement>('textbox', { name: 'Delivery email' });
  const send = canvas.getByRole('button', { name: 'Send update' });
  await userEvent.click(send);
  await expect(input.validity.valueMissing).toBe(true);
  await expect(input).toHaveAttribute('aria-invalid', 'true');
  await expect(canvas.getByRole('status', { name: 'Submitted email' })).toHaveTextContent(
    'No submission',
  );
  await userEvent.type(input, 'invalid');
  await userEvent.click(send);
  await expect(input.validity.typeMismatch).toBe(true);
  await expect(input).toHaveAttribute('aria-errormessage', canvas.getByRole('alert').id);
  await userEvent.clear(input);
  await userEvent.type(input, 'reader@example.com');
  await userEvent.click(send);
  await expect(canvas.getByRole('status', { name: 'Submitted email' })).toHaveTextContent(
    'reader@example.com',
  );
  await expect(input).toHaveValue('reader@example.com');
  await expect(input).not.toHaveAttribute('aria-invalid');
  await expect(input).not.toHaveAttribute('aria-errormessage');
  await expect(input).toHaveAccessibleDescription('Used for delivery updates.');
}

export async function playDisabledAndReadOnly({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const readonly = canvas.getByRole('textbox', { name: 'Read-only value' });
  await userEvent.click(canvas.getByRole('button', { name: 'Before fields' }));
  await userEvent.tab();
  await expect(readonly).toHaveFocus();
  await userEvent.type(readonly, 'replacement');
  await expect(readonly).toHaveValue('Inspectable');
  await expect(canvas.getByRole('textbox', { name: 'Disabled value' })).toBeDisabled();
  await userEvent.tab();
  await expect(canvas.getByRole('button', { name: 'After fields' })).toHaveFocus();
}
