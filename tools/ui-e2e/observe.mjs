import { Blocked } from './core.mjs';
import { setTimeout as delay } from 'node:timers/promises';

export async function readyOrError(ready, scope, timeout) {
  return observeUntil(async () => {
    const error = scope.getByRole('alert').first();
    if (await error.isVisible()) {
      const message = await error.innerText();
      if (message.includes('no verified utterance layout; install the streaming TTS package'))
        throw new Blocked(message);
      throw new Error(`Application reported: ${message}`);
    }
    return (await ready.isVisible()) ? { visible: true } : null;
  }, timeout);
}

// No racing waiters survive a step and act on the next operation's controls.
export async function observeUntil(read, timeout) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const state = await read();
    if (state) return state;
    await delay(Math.min(100, Math.max(1, deadline - Date.now())));
  }
  throw new Error(`UI state was not confirmed within ${timeout} ms`);
}

// A click timeout can occur after dispatch. Observe its outcome once; never retry.
export async function clickOnce(control, observe, options) {
  let ambiguous;
  try {
    await control.click(options);
  } catch (error) {
    if (error.name !== 'TimeoutError') throw error;
    ambiguous = error;
  }
  try {
    return { click: ambiguous ? 'ambiguous-confirmed' : 'confirmed', state: await observe() };
  } catch (error) {
    if (ambiguous) error.message += `; click outcome ambiguous: ${ambiguous.message}`;
    throw error;
  }
}

export async function jobState(scope, label, expected, timeout) {
  return observeUntil(async () => {
    const status = scope.getByRole('status', { name: label, exact: true });
    if (!(await status.isVisible())) return null;
    const text = await status.innerText();
    const state = text.split(' · ').at(-1);
    const alert = scope.getByRole('alert').first();
    const hasAlert = await alert.isVisible();
    if (state === 'ошибка' || hasAlert)
      throw new Error(
        `Job did not reach ${expected.join('/')}: ${text}; ${hasAlert ? await alert.innerText() : ''}`,
      );
    if (expected.includes(state)) return { state, text };
    if (['отменено', 'готово'].includes(state))
      throw new Blocked(
        `Expected ${expected.join('/')}, observed ${text}; requested transition was not exercised`,
      );
    return null;
  }, timeout);
}
