import { useEffect } from 'react';
import { JobContext, type JobDto, type JobStoreState } from '@/entities/job';
import { ProjectProvider, useProjectContext, type Project } from '@/entities/project';
import type { JobHistoryPage } from '@/shared/api/contracts/job';
import { JobQueuePanel } from './JobQueuePanel';

export const queueProject: Project = {
  id: 'queue-project',
  title: 'Chapter one',
  revision: 1,
  status: 'failed',
  source: { kind: 'youtubeUrl', url: 'https://www.youtube.com/watch?v=fixture' },
  metadata: null,
  createdAt: '2026-10-23T10:00:00Z',
  updatedAt: '2026-10-23T10:00:00Z',
};
export const queueJob: JobDto = {
  id: 'queue-job',
  projectId: queueProject.id,
  revision: 1,
  kind: 'dubbing',
  title: 'Subtitle import',
  status: 'running',
  stage: null,
  progress: {
    percent: 42,
    message: 'Importing subtitles',
    currentStep: null,
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: '2026-10-23T10:00:00Z',
  updatedAt: '2026-10-23T10:00:00Z',
};
export const finishedJob: JobDto = {
  ...queueJob,
  id: 'history-one',
  title: 'Earlier completed import',
  status: 'completed',
  progress: { ...queueJob.progress, percent: 100, message: 'Transcript ready' },
};
export const failedJob: JobDto = {
  ...queueJob,
  status: 'failed',
  error: 'Subtitle provider unavailable',
};

export function QueueWorkflow({
  jobs = [],
  switchProject = false,
}: {
  jobs?: JobDto[];
  switchProject?: boolean;
}) {
  const state: JobStoreState = {
    phase: 'ready',
    jobs: Object.fromEntries(jobs.map((job) => [job.id, job])),
    buffer: [],
    pendingRefetch: false,
    generation: 1,
  };
  return (
    <ProjectProvider>
      <SelectQueueProject switchProject={switchProject} />
      <JobContext.Provider value={state}>
        <div className="h-[640px] w-96">
          <JobQueuePanel />
        </div>
      </JobContext.Provider>
    </ProjectProvider>
  );
}
function SelectQueueProject({ switchProject }: { switchProject: boolean }) {
  const { setProject, project } = useProjectContext();
  useEffect(() => {
    setProject(queueProject);
  }, []);
  return (
    <div>
      <p role="status">Selected project: {project?.title ?? 'None'}</p>
      {switchProject && (
        <button
          onClick={() => setProject({ ...queueProject, id: 'other-project', title: 'Chapter two' })}
        >
          Select Chapter two
        </button>
      )}
    </div>
  );
}

export function queueBoundary({ historyFailure = false } = {}) {
  const internal = (
    window as unknown as {
      __TAURI_INTERNALS__: { invoke: (command: string, args?: unknown) => Promise<unknown> };
    }
  ).__TAURI_INTERNALS__;
  const previous = internal.invoke;
  let historyCalls = 0;
  let cancelCalls = 0;
  let retryCalls = 0;
  let cancel: ((error?: string) => void) | undefined;
  let history: ((page: JobHistoryPage) => void) | undefined;
  let retry: (() => void) | undefined;
  internal.invoke = async (command, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    if (command === 'list_job_history_page_cmd') {
      historyCalls++;
      if (historyFailure && historyCalls === 1)
        throw { code: 'IO', message: 'History storage unavailable' };
      if (!historyFailure) return { jobs: [], nextCursor: null };
      if (historyCalls === 2)
        return {
          jobs: [finishedJob],
          nextCursor: { createdAt: finishedJob.createdAt, jobId: finishedJob.id },
        };
      return new Promise<JobHistoryPage>((resolve) => {
        history = resolve;
      });
    }
    if (command === 'cancel_job_cmd') {
      if (args.jobId !== queueJob.id) throw new Error('Unexpected cancellation identity');
      cancelCalls++;
      return new Promise((resolve, reject) => {
        cancel = (error) =>
          error
            ? reject({ code: 'IO', message: error })
            : resolve({ ...queueJob, revision: 2, status: 'cancelled' });
      });
    }
    if (command === 'start_project_mock_pipeline_cmd') {
      if (args.projectId !== queueProject.id) throw new Error('Unexpected retry context');
      retryCalls++;
      return new Promise((resolve) => {
        retry = () =>
          resolve({
            project: { ...queueProject, revision: 2, status: 'processing' },
            job: queueJob,
          });
      });
    }
    return previous(command, payload);
  };
  return {
    calls: () => ({ history: historyCalls, cancel: cancelCalls, retry: retryCalls }),
    finishCancel: (error?: string) => cancel?.(error),
    finishHistory: (page: JobHistoryPage) => history?.(page),
    finishRetry: () => retry?.(),
    restore: () => {
      internal.invoke = previous;
    },
  };
}
