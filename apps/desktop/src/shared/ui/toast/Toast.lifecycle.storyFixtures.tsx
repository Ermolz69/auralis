import { toast, TOAST_EXIT_DURATION } from './toast';

export const FOCUS_TOAST_DURATION = 800;

export async function resetStoryToasts() {
  toast.dismissAll();
  await new Promise((resolve) => globalThis.setTimeout(resolve, TOAST_EXIT_DURATION));
}
