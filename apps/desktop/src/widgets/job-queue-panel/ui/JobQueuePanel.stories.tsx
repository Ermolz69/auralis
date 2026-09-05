import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { JobContext, type JobDto, type JobStoreState } from '@/entities/job';
import { ProjectContext, type Project } from '@/entities/project';
import { JobQueuePanel } from './JobQueuePanel';

const runningJob: JobDto = {
  kind: 'dubbing',
  id: 'job-running',
  revision: 3,
  projectId: 'project-1',
  title: 'Subtitle import',
  status: 'running',
  stage: 'extractOrGenerateTranscript',
  progress: {
    percent: 42,
    message: 'Importing subtitles',
    currentStep: 'subtitle-import',
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:01.000Z',
};

const terminalJobs: JobDto[] = [
  {
    ...runningJob,
    id: 'job-completed',
    title: 'Transcript generation',
    status: 'completed',
    stage: null,
    progress: {
      percent: 100,
      message: 'Transcript ready',
      currentStep: null,
      processedItems: null,
      totalItems: null,
    },
  },
  {
    ...runningJob,
    id: 'job-failed',
    title: 'Local subtitle import',
    status: 'failed',
    stage: null,
    progress: {
      percent: 70,
      message: 'No supported subtitles were found',
      currentStep: null,
      processedItems: null,
      totalItems: null,
    },
    error: 'This source does not support automatic subtitle import.',
  },
  {
    ...runningJob,
    id: 'job-cancelled',
    title: 'Cancelled operation',
    status: 'cancelled',
    stage: null,
    progress: {
      percent: 10,
      message: 'Cancelled',
      currentStep: null,
      processedItems: null,
      totalItems: null,
    },
  },
];

const restartRecoveredJob: JobDto = {
  kind: 'dubbing',
  ...runningJob,
  id: 'job-recovered-after-restart',
  title: 'Subtitle import',
  status: 'failed',
  stage: null,
  progress: {
    percent: 42,
    message: 'Interrupted during subtitle import',
    currentStep: null,
    processedItems: null,
    totalItems: null,
  },
  error: 'Interrupted by application restart',
  updatedAt: '2026-08-02T00:05:00.000Z',
};

const meta = {
  title: 'Widgets/JobQueuePanel/States',
  component: JobQueuePanel,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  render: ({ state }: { state: JobStoreState }) => (
    <div className="h-[640px] w-96">
      <ProjectContext.Provider value={createProjectContext()}>
        <JobContext.Provider value={state}>
          <JobQueuePanel className="border border-muted" />
        </JobContext.Provider>
      </ProjectContext.Provider>
    </div>
  ),
} satisfies Meta<{ state: JobStoreState }>;

export default meta;
type Story = StoryObj<typeof meta>;

export const StaleRunningOperation: Story = {
  args: {
    state: createJobState([runningJob], {
      phase: 'stale',
      pendingRefetch: true,
    }),
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.getByRole('alert')).toHaveTextContent('Job state may be outdated');
    await expect(canvas.getByText('Operation keeps running while you browse')).toBeInTheDocument();
    await expect(canvas.getByRole('heading', { name: 'Active operation' })).toBeInTheDocument();
    await expect(
      canvas.getByRole('progressbar', { name: 'Subtitle import progress' }),
    ).toBeInTheDocument();
  },
};

export const RestartRecoveredFailure: Story = {
  args: {
    state: createJobState([restartRecoveredJob]),
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await expect(canvas.getByRole('alert')).toHaveTextContent(
      'This operation was recovered after restart as interrupted',
    );
    await expect(
      canvas.getByText('Final state: Interrupted during subtitle import'),
    ).toBeInTheDocument();
  },
};

export const TerminalOperations: Story = {
  args: {
    state: createJobState(terminalJobs),
  },
};

function createJobState(
  jobs: JobDto[],
  overrides: Partial<Pick<JobStoreState, 'phase' | 'pendingRefetch'>> = {},
): JobStoreState {
  return {
    phase: overrides.phase ?? 'ready',
    jobs: Object.fromEntries(jobs.map((job) => [job.id, job])),
    buffer: [],
    pendingRefetch: overrides.pendingRefetch ?? false,
    generation: 1,
  };
}

function createProjectContext() {
  const project: Project = {
    id: 'project-1',
    title: 'https://youtube.com/watch?v=123',
    status: 'failed',
    createdAt: '2026-08-02T00:00:00.000Z',
    updatedAt: '2026-08-02T00:00:01.000Z',
    source: { kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=123' },
    metadata: null,
  };

  return {
    selection: project
      ? { status: 'open' as const, project: project }
      : { status: 'closed' as const },
    projectId: project.id,
    project,
    setProject: () => undefined,
    deletingProjectId: null,
    beginProjectDeletion: () => false,
    finishProjectDeletion: () => undefined,
    operationGeneration: 1,
    captureToken: () => ({
      generation: 1,
      projectId: project.id,
    }),
    validateToken: () => true,
  };
}
