import { useLayoutEffect, useRef } from 'react';
import type { OperationToken } from './context';
import { useProjectContext } from './useProjectContext';

type ProjectOperationAttempt = {
  readonly token: OperationToken;
};

export function useProjectOperation() {
  const { projectId, operationGeneration, captureToken, validateToken } = useProjectContext();
  const activeAttempt = useRef<ProjectOperationAttempt | null>(null);

  useLayoutEffect(() => {
    activeAttempt.current = null;
  }, [operationGeneration, projectId]);

  const begin = (): ProjectOperationAttempt | null => {
    if (activeAttempt.current !== null) return null;
    const token = captureToken();
    if (!validateToken(token)) return null;
    const attempt = { token };
    activeAttempt.current = attempt;
    return attempt;
  };

  const isCurrent = (attempt: ProjectOperationAttempt): boolean =>
    activeAttempt.current === attempt && validateToken(attempt.token);

  const finish = (attempt: ProjectOperationAttempt): boolean => {
    if (activeAttempt.current !== attempt) return false;
    activeAttempt.current = null;
    return validateToken(attempt.token);
  };

  return { begin, isCurrent, finish };
}
