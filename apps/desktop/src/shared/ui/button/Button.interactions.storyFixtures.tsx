import { expect, userEvent, within } from 'storybook/test';

export async function playAllVariants({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  for (const name of ['Primary', 'Secondary', 'Ghost', 'Danger']) {
    const button = canvas.getByRole('button', { name });
    await userEvent.click(button);
    await expect(button).toHaveFocus();
    await expect(button).toHaveAttribute('type', 'button');
  }
}

export async function playDisabled({ canvasElement }: { canvasElement: HTMLElement }) {
  const button = within(canvasElement).getByRole('button', { name: 'Disabled Action' });
  await userEvent.click(button);
  await userEvent.tab();
  await expect(button).toBeDisabled();
  await expect(button).not.toHaveFocus();
}

export async function playLoading({ canvasElement }: { canvasElement: HTMLElement }) {
  const button = within(canvasElement).getByRole('button', { name: 'Submitting...' });
  await expect(button).toBeDisabled();
  await expect(button).toHaveAttribute('aria-busy', 'true');
  await userEvent.click(button);
  await expect(button).not.toHaveFocus();
}

export async function playKeyboardAndFormActions({
  canvasElement,
}: {
  canvasElement: HTMLElement;
}) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Before actions' }));
  await userEvent.tab();
  await expect(canvas.getByRole('button', { name: 'Run action' })).toHaveFocus();
  await userEvent.keyboard('{Enter} ');
  await expect(canvas.getByRole('status', { name: 'Action count' })).toHaveTextContent(/^2$/);
  await expect(canvas.getByRole('status', { name: 'Submission count' })).toHaveTextContent(/^0$/);
  await userEvent.click(canvas.getByRole('button', { name: 'Disabled action' }));
  await expect(canvas.getByRole('status', { name: 'Action count' })).toHaveTextContent(/^2$/);
  await userEvent.click(canvas.getByRole('button', { name: 'Before actions' }));
  await userEvent.tab();
  await expect(canvas.getByRole('button', { name: 'Run action' })).toHaveFocus();
  await userEvent.tab();
  await expect(canvas.getByRole('button', { name: 'Submit form' })).toHaveFocus();
  await userEvent.keyboard('{Enter}');
  await expect(canvas.getByRole('status', { name: 'Submission count' })).toHaveTextContent(/^1$/);
}

export async function playBusyAction({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const action = canvas.getByRole('button', { name: 'Run action' });
  await userEvent.dblClick(action);
  await expect(action).toBeDisabled();
  await expect(action).toHaveAttribute('aria-busy', 'true');
  await userEvent.click(action);
  await userEvent.keyboard('{Enter} ');
  await expect(canvas.getByRole('status', { name: 'Action count' })).toHaveTextContent(/^1$/);
  await userEvent.click(canvas.getByRole('button', { name: 'Before actions' }));
  await userEvent.tab();
  await expect(canvas.getByRole('button', { name: 'Submit form' })).toHaveFocus();
  await userEvent.click(canvas.getByRole('button', { name: 'Complete request' }));
  await expect(action).toBeEnabled();
  await expect(action).not.toHaveAttribute('aria-busy');
  await userEvent.click(action);
  await expect(canvas.getByRole('status', { name: 'Action count' })).toHaveTextContent(/^2$/);
  await userEvent.click(canvas.getByRole('button', { name: 'Complete request' }));
}
