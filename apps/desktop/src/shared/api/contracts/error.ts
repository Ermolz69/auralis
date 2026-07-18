export type CommandErrorCode =
  'NOT_FOUND' | 'VALIDATION' | 'CONFLICT' | 'BUSY' | 'REPOSITORY' | 'INTERNAL' | 'CANCELLED';

export interface CommandError {
  code: CommandErrorCode;
  message: string;
}

const VALID_ERROR_CODES = new Set<CommandErrorCode>([
  'NOT_FOUND',
  'VALIDATION',
  'CONFLICT',
  'BUSY',
  'REPOSITORY',
  'INTERNAL',
  'CANCELLED',
]);

export function isCommandError(error: unknown): error is CommandError {
  if (typeof error !== 'object' || error === null) return false;
  if (!('code' in error) || !('message' in error)) return false;
  const typedError = error as Record<string, unknown>;
  return (
    typeof typedError.message === 'string' &&
    typeof typedError.code === 'string' &&
    VALID_ERROR_CODES.has(typedError.code as CommandErrorCode)
  );
}

export function toCommandError(error: unknown): CommandError {
  if (isCommandError(error)) {
    return {
      code: error.code,
      message: error.message,
    };
  }

  return {
    code: 'INTERNAL',
    message: 'An unexpected system error occurred',
  };
}
