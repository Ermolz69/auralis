import { expect, userEvent, within, waitFor } from 'storybook/test';

export async function playSuccessToast({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Show Success Toast' }));
  const notice = await canvas.findByRole('status');
  await expect(notice).toHaveTextContent('Video exported');
  await expect(notice).toHaveTextContent('Your file has been saved to the Desktop.');
  await expect(within(notice).getByRole('button', { name: 'Close toast' })).toBeVisible();
}

export async function playErrorToast({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Show Error Toast' }));
  const notice = await canvas.findByRole('alert');
  await expect(notice).toHaveTextContent('Export failed');
  await expect(notice).toHaveTextContent('Please check your internet connection.');
}

export async function playWarningToast({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Show Warning Toast' }));
  await expect(await canvas.findByRole('alert')).toHaveTextContent('Low disk space');
}

export async function playInfoToast({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Show Info Toast' }));
  await expect(await canvas.findByRole('status')).toHaveTextContent('Update available');
}

export async function playMultipleToasts({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  for (const name of ['Task 1', 'Task 2', 'Task 3']) {
    await userEvent.click(canvas.getByRole('button', { name }));
  }
  const notices = canvasElement.querySelectorAll('[data-toast-phase]');
  await expect(notices).toHaveLength(3);
  await expect(notices[0]).toHaveTextContent('Task 1 completed');
  await expect(notices[1]).toHaveTextContent('Task 2 completed');
  await expect(notices[2]).toHaveTextContent('Task 3 failed');
  await userEvent.dblClick(
    within(notices[1] as HTMLElement).getByRole('button', { name: 'Close toast' }),
  );
  await waitFor(() => expect(notices[1]).not.toBeInTheDocument());
  await expect(notices[0]).toBeVisible();
  await expect(notices[2]).toBeVisible();
}

export async function playLongMessage({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Show Long Message' }));
  const notice = await canvas.findByRole('status');
  await expect(within(notice).getByText('Very long message')).toBeVisible();
  await userEvent.tab();
  await expect(within(notice).getByRole('button', { name: 'Close toast' })).toHaveFocus();
  await userEvent.keyboard('{Enter}');
  await waitFor(() => expect(notice).not.toBeInTheDocument());
}

export async function playWithAction({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Show Action Toast' }));
  const notice = await canvas.findByRole('alert');
  await userEvent.tab();
  await expect(within(notice).getByRole('button', { name: 'Открыть источник' })).toHaveFocus();
  await userEvent.keyboard('{Enter}');
  await expect(canvas.getByRole('status', { name: 'Source actions' })).toHaveTextContent(/^1$/);
  await waitFor(() => expect(notice).not.toBeInTheDocument());
}

export async function playRepeatedAction({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Show action feedback' }));
  const notice = await canvas.findByRole('alert');
  await userEvent.dblClick(within(notice).getByRole('button', { name: 'Retry action' }));
  await expect(canvas.getByRole('status', { name: 'Action activations' })).toHaveTextContent(/^1$/);
  await waitFor(() => expect(notice).not.toBeInTheDocument());
}
