import { toCommandError } from '@/shared/api/contracts';

export function reportReactError(kind: 'caught' | 'uncaught' | 'recoverable', error: unknown) {
  console.error(`Auralis UI ${kind} error`, toCommandError(error));
}
