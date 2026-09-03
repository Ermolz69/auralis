// @vitest-environment jsdom
import { useContext } from 'react';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { listen as listenProject } from '@tauri-apps/api/event';
import { invoke, listen } from '@/shared/api/tauri';
import { NavigationProvider, useNavigation } from '@/shared/router';
import {
  ProjectProvider,
  useProjectContext,
  type Project,
  type ProjectContextType,
} from '@/entities/project';
import { JobContext, useProjectJobs, type JobDto } from '@/entities/job';
import { AppJobProvider } from './app/providers';
import { CurrentStepSummary } from './pages/project/ui/CurrentStepSummary';
import App from './App';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('./pages/home', () => ({ HomePage: () => <h1>Home page</h1> }));
vi.mock('./pages/project', () => ({ ProjectPage: () => <button>Project controls</button> }));
vi.mock('./pages/settings', () => ({ SettingsPage: () => <h1>Settings page</h1> }));

const project: Project = {
  id: 'p1',
  title: 'Selected',
  status: 'draft',
  source: null,
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};
const job: JobDto = {
  kind: 'dubbing',
  id: 'j1',
  projectId: 'p1',
  revision: 1,
  title: 'Old job',
  status: 'running',
  stage: 'importYoutubeSubtitles',
  progress: {
    percent: 10,
    message: 'Working',
    currentStep: null,
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: project.createdAt,
  updatedAt: project.updatedAt,
};
let context: ProjectContextType;
let navigation: ReturnType<typeof useNavigation>;
let projectEvent: (event: { payload: { projectId: string } }) => Promise<void>;
const jobListeners = new Map<string, (event: { payload: unknown }) => void>();
let missing: boolean;
let jobSnapshot: Promise<JobDto[]> | JobDto[];
let creation: Promise<Project> | null;
let observations: { selectedId: string | null; jobs: JobDto[] }[];

function Probe() {
  context = useProjectContext();
  navigation = useNavigation();
  const jobs = useContext(JobContext)!;
  const scoped = useProjectJobs(context.projectId);
  observations.push({
    selectedId: context.projectId,
    jobs: scoped.jobs,
  });
  return (
    <>
      <output data-testid="job-scope">
        {context.projectId ?? 'none'}:{scoped.jobs.length}
      </output>
      <output data-testid="global-jobs">{JSON.stringify(jobs.jobs)}</output>
      <CurrentStepSummary />
    </>
  );
}

beforeEach(() => {
  vi.resetAllMocks();
  jobListeners.clear();
  observations = [];
  missing = false;
  creation = null;
  jobSnapshot = [job];
  vi.spyOn(console, 'warn').mockImplementation(() => {});
  vi.mocked(listenProject).mockImplementation((_name, callback) => {
    projectEvent = callback as typeof projectEvent;
    return Promise.resolve(vi.fn());
  });
  vi.mocked(listen).mockImplementation(async (name, callback) => {
    jobListeners.set(name, callback as (event: { payload: unknown }) => void);
    return () => {
      jobListeners.delete(name);
    };
  });
  vi.mocked(invoke).mockImplementation((async (command: string) => {
    if (command === 'list_projects_cmd') return [];
    if (command === 'create_project_cmd' && creation) return creation;
    if (command === 'list_jobs_cmd') return jobSnapshot;
    if (command === 'get_project_cmd') {
      if (missing) throw { code: 'NOT_FOUND', message: 'Removed elsewhere' };
      return project;
    }
    throw new Error(`Unexpected command: ${command}`);
  }) as typeof invoke);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

async function openWorkspace() {
  render(
    <NavigationProvider>
      <ProjectProvider>
        <AppJobProvider>
          <App />
          <Probe />
        </AppJobProvider>
      </ProjectProvider>
    </NavigationProvider>,
  );
  await act(async () => {
    context.setProject(project);
    navigation.setCurrentView('project');
  });
  await waitFor(() => expect(screen.getByTestId('job-scope').textContent).toBe('p1:1'));
}

it('closes missing selection and clears project selectors while the global queue keeps updating', async () => {
  await openWorkspace();
  const token = context.captureToken();
  const lateJobEvent = jobListeners.get('job-event')!;
  let resolve!: (jobs: JobDto[]) => void;
  jobSnapshot = new Promise((done) => {
    resolve = done;
  });
  act(() => jobListeners.get('job-events-invalidated')!({ payload: {} }));
  missing = true;
  await act(async () => projectEvent({ payload: { projectId: project.id } }));
  expect(context.selection).toEqual({ status: 'closed' });
  expect(context.project).toBeNull();
  expect(context.validateToken(token)).toBe(false);
  expect(navigation.currentView).toBe('home');
  expect(screen.getByRole('heading', { name: 'Home page' })).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Project controls' })).toBeNull();
  expect(
    (
      screen.getByRole('button', {
        name: 'Workspace unavailable without an active project',
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  expect(screen.getByTestId('job-scope').textContent).toBe('none:0');
  await act(async () => {
    resolve([job]);
    lateJobEvent({ payload: { kind: 'progressed', job: { ...job, revision: 2 } } });
  });
  expect(screen.getByTestId('job-scope').textContent).toBe('none:0');
  expect(
    observations.every((value) =>
      value.jobs.every((job) => value.selectedId !== null && job.projectId === value.selectedId),
    ),
  ).toBe(true);
  expect(jobListeners.size).toBe(2);
  expect(JSON.parse(screen.getByTestId('global-jobs').textContent!).j1.revision).toBe(2);
});

it('switches job scope in the same render as the selected project', async () => {
  await openWorkspace();
  fireEvent.click(screen.getByRole('button', { name: /Очередь/ }));
  expect(screen.getByRole('progressbar', { name: 'Old job progress' })).toBeTruthy();
  const listener = jobListeners.get('job-event');
  jobSnapshot = [];
  await act(async () => context.setProject({ ...project, id: 'p2' }));
  expect(screen.getByTestId('job-scope').textContent).toBe('p2:0');
  expect(screen.getByText('No operation is running for this project.')).toBeTruthy();
  expect(
    observations.every((value) => value.jobs.every((job) => job.projectId === value.selectedId)),
  ).toBe(true);
  expect(screen.getByRole('progressbar', { name: 'Old job progress' })).toBeTruthy();
  expect(screen.getByRole('button', { name: /Очередь/ }).textContent).toContain('1');
  expect(jobListeners.get('job-event')).toBe(listener);
  expect(
    vi.mocked(invoke).mock.calls.filter(([command]) => command === 'list_jobs_cmd'),
  ).toHaveLength(1);
  expect(invoke).not.toHaveBeenCalledWith('list_jobs_snapshot_cmd', expect.anything());
  act(() =>
    listener!({
      payload: {
        kind: 'progressed',
        job: {
          ...job,
          revision: 2,
          progress: { ...job.progress, percent: 75 },
        },
      },
    }),
  );
  expect(
    screen.getByRole('progressbar', { name: 'Old job progress' }).getAttribute('aria-valuenow'),
  ).toBe('75');
  await act(async () => context.setProject(null));
  expect(screen.getByRole('button', { name: /Очередь/ }).textContent).toContain('1');
  expect(screen.getByRole('progressbar', { name: 'Old job progress' })).toBeTruthy();
  expect(screen.getByTestId('job-scope').textContent).toBe('none:0');
  act(() =>
    listener!({
      payload: { kind: 'completed', job: { ...job, revision: 3, status: 'completed' } },
    }),
  );
  expect(screen.queryByRole('progressbar', { name: 'Old job progress' })).toBeNull();
  expect(screen.getByRole('list', { name: 'Operation history' }).textContent).toContain('Old job');
});

it('returns home if the selected project disappears while settings are visible', async () => {
  await openWorkspace();
  act(() => navigation.setCurrentView('settings'));
  expect(screen.getByRole('heading', { name: 'Settings page' })).toBeTruthy();
  missing = true;
  await act(async () => projectEvent({ payload: { projectId: project.id } }));
  expect(navigation.currentView).toBe('home');
  expect(screen.getByRole('heading', { name: 'Home page' })).toBeTruthy();
});

it('does not navigate back into a project when an invalidated create operation completes', async () => {
  await openWorkspace();
  let resolve!: (value: Project) => void;
  creation = new Promise((done) => {
    resolve = done;
  });
  fireEvent.change(screen.getByRole('textbox', { name: 'Project name' }), {
    target: { value: 'New project' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
  expect(invoke).toHaveBeenCalledWith('create_project_cmd', { title: 'New project' });
  missing = true;
  await act(async () => projectEvent({ payload: { projectId: project.id } }));
  await act(async () => resolve({ ...project, id: 'created' }));
  expect(context.selection.status).toBe('closed');
  expect(navigation.currentView).toBe('home');
  expect(screen.getByTestId('job-scope').textContent).toBe('none:0');
});
