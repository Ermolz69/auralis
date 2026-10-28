import { expect, userEvent, within, waitFor } from 'storybook/test';

export async function playSwipeFeedback({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole('button', { name: 'Show action feedback' }));
  const notice = await canvas.findByRole('alert');
  const box = notice.getBoundingClientRect();
  const start = { clientX: box.left + 100, clientY: box.top + 12 };
  const end = { ...start, clientX: start.clientX + 120 };
  await userEvent.pointer([
    { target: notice, coords: start, keys: '[MouseLeft>]' },
    { coords: end },
  ]);
  notice.dispatchEvent(new PointerEvent('pointercancel', { pointerId: 1, bubbles: true }));
  await userEvent.pointer({ keys: '[/MouseLeft]' });
  await expect(notice).toHaveAttribute('data-toast-phase', 'visible');
  await expect(notice).toHaveAttribute('data-toast-dragging', 'false');
  await expect(notice.style.transform).toBe('');
  await userEvent.pointer([
    { target: notice, coords: start, keys: '[MouseLeft>]' },
    { coords: end },
    { keys: '[/MouseLeft]' },
  ]);
  await waitFor(() => expect(notice).not.toBeInTheDocument());
  await expect(canvas.getByRole('status', { name: 'Action activations' })).toHaveTextContent(/^0$/);
}
