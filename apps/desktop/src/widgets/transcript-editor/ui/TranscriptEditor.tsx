import { useContext } from 'react';
import { useTranscript } from '@/entities/transcript';
import { useProjectContext } from '@/entities/project';
import { formatJobStatus, isActiveJobStatus, JobContext, selectProjectJobs } from '@/entities/job';
import { TranscriptPanelView } from './TranscriptPanelView';

export const TranscriptEditor = () => {
  const { projectId, project } = useProjectContext();
  const { transcript, isLoading, error, refetch } = useTranscript(projectId);
  const jobState = useContext(JobContext);
  const sourceKind = project?.source?.kind;
  const activeTranscriptJob = selectProjectJobs(jobState?.jobs ?? {}, projectId).find((job) =>
    isActiveJobStatus(job.status),
  );
  const activeJobStatus = activeTranscriptJob ? formatJobStatus(activeTranscriptJob) : null;

  return (
    <TranscriptPanelView
      projectId={projectId}
      sourceKind={sourceKind ?? null}
      transcript={transcript}
      isLoading={isLoading}
      error={error}
      activeJobStatus={activeJobStatus}
      onRefetch={refetch}
    />
  );
};
