export type CommandErrorCode = 'NOT_FOUND' | 'VALIDATION' | 'REPOSITORY' | 'INTERNAL';

export interface CommandError {
  code: CommandErrorCode;
  message: string;
}

export function isCommandError(error: unknown): error is CommandError {
  return typeof error === 'object' && error !== null && 'code' in error && 'message' in error;
}
