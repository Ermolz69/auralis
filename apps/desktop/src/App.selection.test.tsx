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
import { JobContext, type JobDto } from '@/entities/job';
import { AppJobProvider } from './app/providers';
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
let observations: { selectedId: string | null; scope: string | null; jobs: number }[];

function Probe() {
  context = useProjectContext();
  navigation = useNavigation();
  const jobs = useContext(JobContext)!;
  observations.push({
    selectedId: context.projectId,
    scope: jobs.scopeProjectId,
    jobs: Object.keys(jobs.jobs).length,
  });
  return (
    <output data-testid="job-scope">
      {jobs.scopeProjectId ?? 'none'}:{Object.keys(jobs.jobs).length}
    </output>
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
    if (command === 'list_jobs_snapshot_cmd') return jobSnapshot;
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

it('closes missing selection, navigates home and clears job scope before any stale UI render', async () => {
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
    observations.every(
      (value) =>
        value.scope === value.selectedId && (value.selectedId !== null || value.jobs === 0),
    ),
  ).toBe(true);
  expect(jobListeners.size).toBe(0);
});

it('switches job scope in the same render as the selected project', async () => {
  await openWorkspace();
  jobSnapshot = [];
  await act(async () => context.setProject({ ...project, id: 'p2' }));
  expect(screen.getByTestId('job-scope').textContent).toBe('p2:0');
  expect(observations.every((value) => value.scope === value.selectedId)).toBe(true);
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
