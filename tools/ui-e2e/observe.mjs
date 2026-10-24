import { Blocked } from './core.mjs';

export async function readyOrError(ready, scope, timeout) {
  const error = scope.getByRole('alert').first();
  await Promise.race([
    ready.waitFor({ state: 'visible', timeout }),
    error.waitFor({ state: 'visible', timeout }).then(async () => {
      const message = await error.innerText();
      if (message.includes('no verified utterance layout; install the streaming TTS package'))
        throw new Blocked(message);
      throw new Error(`Application reported: ${message}`);
    }),
  ]);
}
