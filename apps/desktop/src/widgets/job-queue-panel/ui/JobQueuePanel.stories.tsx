import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { JobContext, type JobDto, type JobStoreState } from '@/entities/job';
import { JobQueuePanel } from './JobQueuePanel';

const runningJob: JobDto = {
  id: 'job-running',
  revision: 3,
  projectId: 'project-1',
  title: 'Subtitle import',
  status: 'running',
  stage: 'importYoutubeSubtitles',
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

const meta = {
  title: 'Widgets/JobQueuePanel/States',
  component: JobQueuePanel,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  render: ({ state }: { state: JobStoreState }) => (
    <div className="h-[640px] w-96">
      <JobContext.Provider value={state}>
        <JobQueuePanel className="border border-muted" />
      </JobContext.Provider>
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
    await expect(
      canvas.getByRole('progressbar', { name: 'Subtitle import progress' }),
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
    scopeProjectId: 'project-1',
    jobs: Object.fromEntries(jobs.map((job) => [job.id, job])),
    buffer: [],
    pendingRefetch: overrides.pendingRefetch ?? false,
    generation: 1,
  };
}
