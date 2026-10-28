import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { ProjectListPanel } from './ProjectListPanel';

const mocks = vi.hoisted(() => ({
  createProject: vi.fn(),
  setProject: vi.fn(),
  captureToken: vi.fn(),
  validateToken: vi.fn(),
  setCurrentView: vi.fn(),
  setPipelineStep: vi.fn(),
  retryLoad: vi.fn(),
  retry: vi.fn(),
  warning: vi.fn(),
  error: vi.fn(),
  success: vi.fn(),
  pins: [] as Array<{ id: string; title: string; revision: number }>,
  persistence: { loadStatus: 'ready', unsavedIds: [] as string[] },
}));
vi.mock('@/entities/project', () => ({
  createProject: mocks.createProject,
  useProjectContext: () => ({
    projectId: 'selected',
    setProject: mocks.setProject,
    captureToken: mocks.captureToken,
    validateToken: mocks.validateToken,
  }),
  usePinPersistence: () => ({
    ...mocks.persistence,
    retryLoad: mocks.retryLoad,
    retry: mocks.retry,
  }),
}));
vi.mock('@/shared/router', () => ({
  useNavigation: () => ({
    currentView: 'project',
    setCurrentView: mocks.setCurrentView,
    setPipelineStep: mocks.setPipelineStep,
  }),
}));
vi.mock('../../model/usePinnedProjects', () => ({ usePinnedProjects: () => mocks.pins }));
vi.mock('@/shared/ui/toast', () => ({
  toast: { warning: mocks.warning, error: mocks.error, success: mocks.success },
}));
beforeEach(() => {
  vi.clearAllMocks();
  mocks.pins = [];
  mocks.persistence = { loadStatus: 'ready', unsavedIds: [] };
  mocks.captureToken.mockReturnValue('context-token');
  mocks.validateToken.mockReturnValue(true);
  mocks.createProject.mockResolvedValue({ id: 'created', title: 'Created', revision: 1 });
});
afterEach(cleanup);

it('requires a name, focuses validation and supports clearing without creating', () => {
  render(<ProjectListPanel height={200} />);
  fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
  const input = screen.getByRole('textbox', { name: 'Project name' });
  expect(input.getAttribute('aria-invalid')).toBe('true');
  expect(document.activeElement).toBe(input);
  expect(mocks.warning).toHaveBeenCalledWith('Укажите название проекта');
  expect(mocks.createProject).not.toHaveBeenCalled();
  fireEvent.change(input, { target: { value: 'Draft' } });
  expect(input.getAttribute('aria-invalid')).toBe('false');
  fireEvent.click(screen.getByRole('button', { name: 'Clear project name' }));
  expect((input as HTMLInputElement).value).toBe('');
  expect(document.activeElement).toBe(input);
});

it('creates a trimmed project on form submit, blocks repeated clicks and navigates after acceptance', async () => {
  let resolve!: (project: unknown) => void;
  mocks.createProject.mockReturnValueOnce(
    new Promise((yes) => {
      resolve = yes;
    }),
  );
  render(<ProjectListPanel height={200} />);
  fireEvent.change(screen.getByRole('textbox'), { target: { value: '  Created  ' } });
  fireEvent.submit(screen.getByRole('textbox').closest('form')!);
  fireEvent.submit(screen.getByRole('textbox').closest('form')!);
  fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
  expect(mocks.createProject).toHaveBeenCalledExactlyOnceWith('Created');
  const project = { id: 'created', title: 'Created', revision: 1 };
  await act(async () => {
    resolve(project);
  });
  expect(mocks.setProject).toHaveBeenCalledWith(project);
  expect(mocks.setPipelineStep).toHaveBeenCalledWith('subtitles');
  expect(mocks.setCurrentView).toHaveBeenCalledWith('project');
  expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('');
});

it('retains the draft and presents a retryable creation error', async () => {
  mocks.createProject.mockRejectedValueOnce({ code: 'IO', message: 'Save unavailable. Retry.' });
  render(<ProjectListPanel height={200} />);
  fireEvent.change(screen.getByRole('textbox'), { target: { value: 'Keep draft' } });
  fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
  await waitFor(() => expect(mocks.error).toHaveBeenCalledWith('Save unavailable. Retry.'));
  expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('Keep draft');
  expect(mocks.setProject).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
  await waitFor(() => expect(mocks.setProject).toHaveBeenCalledOnce());
});

it.each([true, false])(
  'ignores late %s completion after the project selection changes',
  async (success) => {
    let resolve!: (value: unknown) => void;
    let reject!: (reason: unknown) => void;
    mocks.createProject.mockReturnValueOnce(
      new Promise((yes, no) => {
        resolve = yes;
        reject = no;
      }),
    );
    render(<ProjectListPanel height={200} />);
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'Draft' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
    mocks.validateToken.mockReturnValue(false);
    await act(async () => {
      if (success) resolve({ id: 'late' });
      else reject(new Error('Late failure'));
    });
    expect(mocks.setProject).not.toHaveBeenCalled();
    expect(mocks.error).not.toHaveBeenCalled();
    expect(mocks.success).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Create project' }).hasAttribute('disabled')).toBe(
      false,
    );
  },
);

it('refuses admission for an invalid selection token and exposes pin retry actions', () => {
  mocks.validateToken.mockReturnValue(false);
  mocks.persistence = { loadStatus: 'error', unsavedIds: ['a', 'b'] };
  mocks.pins = [{ id: 'selected', title: 'Pinned project', revision: 2 }];
  render(<ProjectListPanel height={200} />);
  fireEvent.change(screen.getByRole('textbox'), { target: { value: 'Draft' } });
  fireEvent.click(screen.getByRole('button', { name: 'Create project' }));
  expect(mocks.createProject).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Pins unavailable. Retry loading pins' }));
  expect(mocks.retryLoad).toHaveBeenCalledOnce();
  fireEvent.click(screen.getByRole('button', { name: 'Pins not yet saved. Retry saving pins' }));
  expect(mocks.retry.mock.calls.map(([id]) => id)).toEqual(['a', 'b']);
  fireEvent.click(screen.getByRole('button', { name: 'Pinned project' }));
  expect(mocks.setProject).toHaveBeenCalledWith(mocks.pins[0]);
  expect(mocks.setPipelineStep).toHaveBeenCalledWith('source');
  expect(mocks.setCurrentView).toHaveBeenCalledWith('project');
});
