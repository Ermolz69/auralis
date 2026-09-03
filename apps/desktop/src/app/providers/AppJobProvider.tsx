import type { ReactNode } from 'react';
import { JobProvider } from '@/entities/job';

export function AppJobProvider({ children }: { children: ReactNode }) {
  return <JobProvider>{children}</JobProvider>;
}
