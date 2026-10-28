import { useEffect, useRef } from 'react';
import { LibraryProvider, useLibrary } from '@/entities/library';
import { StudioShell } from './StudioShell';
import { JobContext, type JobStoreState } from '@/entities/job';
import { studioProject } from './StudioShell.ipc.storyFixtures';

const activeJobs: JobStoreState = {
  phase: 'ready',
  buffer: [],
  pendingRefetch: false,
  generation: 0,
  jobs: Object.fromEntries(
    ['running', 'pending', 'cancelling'].map((status, index) => [
      `job-${index}`,
      {
        id: `job-${index}`,
        projectId: null,
        kind: 'speech',
        revision: 1,
        title: ['Подготовка субтитров', 'Синтез речи · Глава 2', 'Перевод · Глава 3'][index],
        status: status as 'running' | 'pending' | 'cancelling',
        stage: null,
        progress: {
          percent: [42, 0, 73][index],
          message: '',
          currentStep: null,
          processedItems: null,
          totalItems: null,
        },
        error: null,
        createdAt: '2026-10-24T10:00:00Z',
        updatedAt: '2026-10-24T10:00:00Z',
      },
    ]),
  ),
};

export function StudioStory({
  project = false,
  busy = false,
}: {
  project?: boolean;
  empty?: boolean;
  busy?: boolean;
  failRefresh?: boolean;
}) {
  return (
    <LibraryProvider>
      <OpenInitialProject enabled={project} />
      <JobContext.Provider value={busy ? activeJobs : null}>
        <StudioShell />
      </JobContext.Provider>
    </LibraryProvider>
  );
}

function OpenInitialProject({ enabled }: { enabled: boolean }) {
  const { loading, openProject } = useLibrary();
  const opened = useRef(false);
  useEffect(() => {
    if (enabled && !loading && !opened.current) {
      opened.current = true;
      void openProject(studioProject.id);
    }
  }, [enabled, loading, openProject]);
  return null;
}
