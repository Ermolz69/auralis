import { expect, userEvent, within, waitFor } from 'storybook/test';
import { FOCUS_TOAST_DURATION } from './Toast.lifecycle.storyFixtures';
import { TOAST_EXIT_DURATION } from './toast';

export async function playKeyboardFocusPausesDismissal({
  canvasElement,
}: {
  canvasElement: HTMLElement;
}) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Show action feedback' }));
  const notice = await canvas.findByRole('alert');
  const action = within(notice).getByRole('button', { name: 'Retry action' });
  await userEvent.tab();
  await userEvent.tab();
  await expect(action).toHaveFocus();
  await userEvent.tab();
  await expect(within(notice).getByRole('button', { name: 'Close toast' })).toHaveFocus();
  await new Promise((resolve) =>
    globalThis.setTimeout(resolve, FOCUS_TOAST_DURATION + TOAST_EXIT_DURATION + 100),
  );
  await expect(notice).toBeVisible();
  await expect(canvas.getByRole('status', { name: 'Action activations' })).toHaveTextContent('0');
  await userEvent.click(canvas.getByRole('button', { name: 'Return to page' }));
  await waitFor(() => expect(notice).not.toBeInTheDocument(), { timeout: 2000 });
}

export async function playHoverPausesDismissal({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Show action feedback' }));
  const notice = await canvas.findByRole('alert');
  await userEvent.hover(notice);
  await new Promise((resolve) =>
    globalThis.setTimeout(resolve, FOCUS_TOAST_DURATION + TOAST_EXIT_DURATION + 100),
  );
  await expect(notice).toBeVisible();
  await userEvent.unhover(notice);
  await waitFor(() => expect(notice).not.toBeInTheDocument(), { timeout: 2000 });
}
