import { expect, userEvent, within } from 'storybook/test';

export async function playDisabled({ canvasElement }: { canvasElement: HTMLElement }) {
  const field = within(canvasElement).getByRole('combobox', { name: 'Select Fruit' });
  await userEvent.selectOptions(field, 'banana');
  await expect(field).toBeDisabled();
  await expect(field).toHaveValue('');
}

export async function playErrorState({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const field = canvas.getByRole('combobox', { name: 'Select Fruit' });
  await expect(field).toHaveAttribute('aria-invalid', 'true');
  await expect(field).toHaveAttribute('aria-errormessage', canvas.getByRole('alert').id);
  await expect(field).toHaveAccessibleDescription(
    'This value controls the next import step. Select a source type to continue.',
  );
}

export async function playGroupedOptions({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const field = canvas.getByRole('combobox', { name: 'Color theme' });
  await userEvent.click(canvas.getByText('Color theme', { selector: 'label' }));
  await expect(field).toHaveFocus();
  await expect(canvas.getByRole('group', { name: 'Light themes' })).toBeInTheDocument();
  await expect(canvas.getByRole('group', { name: 'Dark themes' })).toBeInTheDocument();
  await userEvent.selectOptions(field, 'dark');
  await expect(field).toHaveValue('dark');
}

export async function playControlledSelection({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const field = canvas.getByRole('combobox', { name: 'Accepted source' });
  await userEvent.click(canvas.getByRole('button', { name: 'Before choice' }));
  await userEvent.tab();
  await expect(field).toHaveFocus();
  await expect(field).toHaveValue('0');
  await userEvent.selectOptions(field, 'subtitles');
  await expect(field).toHaveValue('0');
  await expect(canvas.getByRole('status', { name: 'Requested source' })).toHaveTextContent(
    'subtitles',
  );
  await expect(canvas.getByRole('status', { name: 'Change requests' })).toHaveTextContent(/^1$/);
  await userEvent.selectOptions(field, 'missing');
  await expect(canvas.getByRole('status', { name: 'Change requests' })).toHaveTextContent(/^1$/);
  await userEvent.click(canvas.getByRole('button', { name: 'Accept choice' }));
  await expect(field).toHaveValue('subtitles');
  await expect(canvas.getByRole('status', { name: 'Requested source' })).toHaveTextContent('None');
}

export async function playValidationAndReset({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const field = canvas.getByRole<HTMLSelectElement>('combobox', { name: 'Required source' });
  await userEvent.click(canvas.getByRole('button', { name: 'Continue' }));
  await expect(field.validity.valueMissing).toBe(true);
  await expect(field).toHaveAttribute('aria-invalid', 'true');
  await userEvent.selectOptions(field, 'text');
  await userEvent.click(canvas.getByRole('button', { name: 'Continue' }));
  await expect(canvas.getByRole('status', { name: 'Submitted source' })).toHaveTextContent(
    /^text$/,
  );
  await expect(field).not.toHaveAttribute('aria-invalid');
  await expect(field).toHaveAccessibleDescription('Select an available source.');
  await userEvent.click(canvas.getByRole('button', { name: 'Reset choice' }));
  await expect(field).toHaveValue('');
  await expect(canvas.getByRole('status', { name: 'Submitted source' })).toHaveTextContent(
    'No submission',
  );
  await userEvent.click(field);
  await userEvent.tab();
  await expect(canvas.getByRole('button', { name: 'Continue' })).toHaveFocus();
  await expect(canvas.getByRole('combobox', { name: 'Disabled source' })).toBeDisabled();
}
