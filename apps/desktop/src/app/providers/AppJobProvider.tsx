import type { ReactNode } from 'react';
import { useProjectContext } from '@/entities/project';
import { JobProvider } from '@/entities/job';

export function AppJobProvider({ children }: { children: ReactNode }) {
  const { selection } = useProjectContext();
  const projectId = selection.status === 'open' ? selection.project.id : null;
  return <JobProvider projectId={projectId}>{children}</JobProvider>;
}
