import type { ReactNode } from 'react';
import { useProjectContext } from '@/entities/project';
import { JobProvider } from '@/entities/job';

export function AppJobProvider({ children }: { children: ReactNode }) {
  const { projectId } = useProjectContext();
  return <JobProvider projectId={projectId}>{children}</JobProvider>;
}
