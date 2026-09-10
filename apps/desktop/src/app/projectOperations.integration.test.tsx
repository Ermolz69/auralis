// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { open } from '@tauri-apps/plugin-dialog';
import {
  ProjectProvider,
  createProject,
  createProjectFromYoutube,
  startProjectMockPipeline,
  useProjectContext,
  type CreateProjectResponse,
  type Project,
  type ProjectContextType,
} from '@/entities/project';
import { importLocalMedia } from '@/entities/media';
import { ImportLocalMediaButton } from '@/features/import-local-media';
import { PasteYoutubeLink } from '@/features/paste-youtube-link';
import { RunDubbing } from '@/features/run-dubbing';
import { NavigationProvider, useNavigation } from '@/shared/router';
import { listen } from '@/shared/api/tauri';
import { toast } from '@/shared/ui/toast';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@/shared/ui/toast', () => ({ toast: { error: vi.fn() } }));
vi.mock('@/entities/project', async (original) => ({
  ...(await original<typeof import('@/entities/project')>()),
  createProject: vi.fn(),
  createProjectFromYoutube: vi.fn(),
  startProjectMockPipeline: vi.fn(),
}));
vi.mock('@/entities/media', async (original) => ({
  ...(await original<typeof import('@/entities/media')>()),
  importLocalMedia: vi.fn(),
}));

type Action = 'local' | 'youtube' | 'subtitles';
let context: ProjectContextType;
let navigation: ReturnType<typeof useNavigation>;

const project = (id: string): Project => ({
  id,
  title: id,
  status: 'ready_for_processing',
  source: { kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=test' },
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
});

function Probe() {
  context = useProjectContext();
  navigation = useNavigation();
  return null;
}

function renderAction(action: Action, selected: Project | null = project('a')) {
  const rendered = render(
    <ProjectProvider>
      <NavigationProvider>
        <Probe />
        {action === 'local' && <ImportLocalMediaButton />}
        {action === 'youtube' && <PasteYoutubeLink />}
        {action === 'subtitles' && <RunDubbing />}
      </NavigationProvider>
    </ProjectProvider>,
  );
  act(() => context.setProject(selected));
  return rendered;
}

function startAction(action: Action) {
  if (action === 'youtube') {
    fireEvent.change(screen.getByRole('textbox', { name: 'YouTube URL' }), {
      target: { value: 'https://youtube.com/watch?v=test' },
    });
  }
  const button = screen.getByRole<HTMLButtonElement>('button', {
    name:
      action === 'local'
        ? 'Import local video'
        : action === 'youtube'
          ? 'Add from YouTube'
          : 'Import subtitles',
  });
  act(() => {
    button.click();
    button.click();
  });
  return button;
}

function mockRequests(action: Action, promises: Promise<Project>[]) {
  const next = vi.fn(() => {
    const promise = promises.shift();
    if (!promise) throw new Error('Unexpected duplicate operation');
    return promise;
  });
  if (action === 'local') vi.mocked(importLocalMedia).mockImplementation(next);
  if (action === 'youtube') vi.mocked(createProjectFromYoutube).mockImplementation(next);
  if (action === 'subtitles') {
    vi.mocked(startProjectMockPipeline).mockImplementation(
      async () => ({ project: await next() }) as CreateProjectResponse,
    );
  }
  return next;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(listen).mockResolvedValue(() => {});
  vi.mocked(open).mockResolvedValue('C:\\media\\video.mp4');
  vi.mocked(createProject).mockResolvedValue(project('created'));
});

afterEach(cleanup);

describe.each<Action>(['local', 'youtube', 'subtitles'])('%s project operation', (action) => {
  it.each([
    ['switch', 'success'],
    ['switch', 'error'],
    ['deletion', 'success'],
    ['deletion', 'error'],
  ] as const)('keeps a new attempt busy after %s and a late %s', async (transition, outcome) => {
    const old = deferred<Project>();
    const current = deferred<Project>();
    const requests = mockRequests(action, [old.promise, current.promise]);
    renderAction(action);
    const firstButton = startAction(action);
    await waitFor(() => expect(requests).toHaveBeenCalledTimes(1));
    expect(firstButton.disabled).toBe(true);
    if (action === 'local') expect(open).toHaveBeenCalledTimes(1);

    if (transition === 'deletion') {
      act(() => {
        context.beginProjectDeletion('a');
      });
      expect(firstButton.disabled).toBe(true);
      act(() => {
        context.setProject(null);
        context.finishProjectDeletion('a');
      });
    }
    const selected = project('b');
    act(() => context.setProject(selected));
    expect(firstButton.disabled).toBe(false);
    const button = startAction(action);
    await waitFor(() => expect(requests).toHaveBeenCalledTimes(2));
    if (action === 'local') expect(open).toHaveBeenCalledTimes(2);

    await act(async () => {
      if (outcome === 'success') old.resolve(project('stale'));
      else old.reject({ code: 'REPOSITORY', message: 'Stale operation failed' });
      await old.promise.catch(() => {});
    });

    expect(context.project).toEqual(selected);
    expect(navigation.currentView).toBe('home');
    expect(button.disabled).toBe(true);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(toast.error).not.toHaveBeenCalled();
    if (action === 'youtube') {
      expect(screen.getByRole<HTMLInputElement>('textbox', { name: 'YouTube URL' }).value).toBe(
        'https://youtube.com/watch?v=test',
      );
    }

    const imported = { ...selected, title: 'Current result' };
    await act(async () => {
      current.resolve(imported);
      await current.promise;
    });
    await waitFor(() => expect(context.project).toEqual(imported));
    expect(button.disabled).toBe(action === 'youtube');
    expect(screen.queryByRole('status')).toBeNull();
    expect(requests).toHaveBeenCalledTimes(2);
    expect(navigation.currentView).toBe(action === 'subtitles' ? 'home' : 'project');
  });
});

it.each<Action>(['local', 'youtube'])(
  'finishes %s before selecting its newly created project',
  async (action) => {
    const pending = deferred<Project>();
    mockRequests(action, [pending.promise]);
    renderAction(action, null);
    const button = startAction(action);
    const created = project('created');
    await act(async () => {
      pending.resolve(created);
      await pending.promise;
    });
    await waitFor(() => expect(context.project).toEqual(created));
    expect(button.disabled).toBe(action === 'youtube');
    expect(screen.queryByRole('status')).toBeNull();
    expect(navigation.currentView).toBe('project');
    expect(navigation.pipelineStep).toBe('source');
  },
);

it('allows two instances to start independently for the same project', async () => {
  const first = deferred<Project>();
  const second = deferred<Project>();
  const requests = mockRequests('subtitles', [first.promise, second.promise]);
  render(
    <ProjectProvider>
      <NavigationProvider>
        <Probe />
        <RunDubbing label="First import" />
        <RunDubbing label="Second import" />
      </NavigationProvider>
    </ProjectProvider>,
  );
  act(() => context.setProject(project('a')));
  const buttons = screen.getAllByRole<HTMLButtonElement>('button');
  act(() => {
    buttons[0].click();
    buttons[1].click();
  });
  expect(requests).toHaveBeenCalledTimes(2);
  await act(async () => {
    first.resolve(project('a'));
    await first.promise;
  });
  expect(buttons[0].disabled).toBe(false);
  expect(buttons[1].disabled).toBe(true);
  await act(async () => {
    second.resolve(project('a'));
    await second.promise;
  });
  expect(buttons[1].disabled).toBe(false);
});
