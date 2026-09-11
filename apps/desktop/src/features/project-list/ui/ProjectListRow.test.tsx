// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import {
  getProjectAvatar,
  getProjectPreferences,
  setProjectAvatar,
  type Project,
} from '@/entities/project';
import { toast } from '@/shared/ui/toast';
import { ProjectListRow } from './ProjectListRow';

vi.mock('@/entities/project', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/entities/project')>()),
  getProjectAvatar: vi.fn().mockResolvedValue({ dataUrl: null, initialized: true }),
  setProjectAvatar: vi.fn().mockResolvedValue({ dataUrl: null, initialized: true }),
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: { error: vi.fn(), warning: vi.fn() },
}));
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke }));
let pinRevision = 0;

const project: Project = {
  revision: 1,
  id: 'project-1',
  title: 'C:\\Users\\person\\Videos\\private-folder\\clip.mp4',
  status: 'failed',
  source: {
    kind: 'externalLocalFile',
    path: 'C:\\Users\\person\\Videos\\private-folder\\clip.mp4',
  },
  metadata: null,
  createdAt: '2026-08-02T00:00:00.000Z',
  updatedAt: '2026-08-02T00:00:01.000Z',
};

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
  invoke.mockImplementation(
    async (command: string, args?: { projectId: string; pinned: boolean }) => {
      if (command === 'get_project_pins_cmd') return { migrated: true, entries: [] };
      if (command === 'set_project_pin_cmd') return { ...args, revision: ++pinRevision };
      return null;
    },
  );
});

afterEach(() => cleanup());

function renderRow(id = project.id) {
  return render(
    <ProjectListRow
      project={{ ...project, id }}
      isDeleting={false}
      isAnyDeleting={false}
      openButtonRef={vi.fn()}
      deleteButtonRef={vi.fn()}
      onOpen={vi.fn()}
      onDelete={vi.fn()}
    />,
  );
}

describe('ProjectListRow', () => {
  it.each([true, false])(
    'retains a failed pin across remount (failure before unmount: %s)',
    async (beforeUnmount) => {
      const id = `pin-remount-${beforeUnmount}`;
      let fail!: (error: Error) => void;
      invoke.mockImplementation(async (command: string, args?: { pinned: boolean }) => {
        if (command === 'get_project_pins_cmd') return { migrated: true, entries: [] };
        if (command === 'set_project_pin_cmd')
          return new Promise((_resolve, reject) => {
            fail = reject;
          });
        return args;
      });
      const first = renderRow(id);
      fireEvent.contextMenu(screen.getByRole('button', { name: /^Open clip\.mp4/ }));
      fireEvent.click(screen.getByRole('menuitem', { name: 'Закрепить' }));
      await waitFor(() => expect(fail).toBeDefined());
      if (beforeUnmount) {
        fail(new Error('write unavailable'));
        await screen.findByRole('button', { name: 'Retry saving pin' });
      }
      first.unmount();
      if (!beforeUnmount) fail(new Error('write unavailable'));
      renderRow(id);
      await screen.findByRole('button', { name: 'Retry saving pin' });
      invoke.mockImplementation(async (command: string, args?: { pinned: boolean }) =>
        command === 'get_project_pins_cmd'
          ? { migrated: true, entries: [] }
          : { projectId: id, pinned: args!.pinned, revision: 1 },
      );
      fireEvent.click(screen.getByRole('button', { name: 'Retry saving pin' }));
      await waitFor(() =>
        expect(screen.queryByRole('button', { name: 'Retry saving pin' })).toBeNull(),
      );
      await waitFor(() => expect(screen.queryByText('Saving pin…')).toBeNull());
      expect(getProjectPreferences(id).pinned).toBe(true);
    },
  );

  it('queues a second real toggle while the first save is pending', async () => {
    const id = 'rapid-pin-row';
    let acknowledge!: () => void;
    let saved = { projectId: id, pinned: false, revision: 0 };
    invoke.mockImplementation(
      async (command: string, args?: { pinned: boolean; expectedRevision: number }) => {
        if (command === 'get_project_pins_cmd')
          return { migrated: true, entries: saved.revision ? [saved] : [] };
        if (command === 'set_project_pin_cmd') {
          expect(args!.expectedRevision).toBe(saved.revision);
          if (!saved.revision)
            await new Promise<void>((resolve) => {
              acknowledge = resolve;
            });
          saved = { ...saved, pinned: args!.pinned, revision: saved.revision + 1 };
          return saved;
        }
        return null;
      },
    );
    renderRow(id);
    const open = screen.getByRole('button', { name: /^Open clip\.mp4/ });
    fireEvent.contextMenu(open);
    fireEvent.click(screen.getByRole('menuitem', { name: 'Закрепить' }));
    await waitFor(() => expect(acknowledge).toBeDefined());
    fireEvent.contextMenu(open);
    fireEvent.click(screen.getByRole('menuitem', { name: 'Открепить' }));
    expect(getProjectPreferences(id).pinned).toBe(false);
    acknowledge();
    await waitFor(() => expect(saved).toEqual({ projectId: id, pinned: false, revision: 2 }));
    await waitFor(() => expect(screen.queryByText('Saving pin…')).toBeNull());
  });

  it('uses safe title, status, and source labels without exposing full local paths', () => {
    renderRow();

    expect(screen.getAllByText('clip.mp4')).toHaveLength(2);
    expect(screen.getByText('Needs attention')).not.toBeNull();
    expect(screen.queryByText('failed')).toBeNull();
    expect(document.body.innerHTML).not.toContain('Users\\person');
    expect(
      screen.getByRole('button', {
        name: 'Open clip.mp4. Status: Needs attention. Source: clip.mp4',
      }),
    ).not.toBeNull();
  });

  it('loads a saved avatar and lets the user remove it', async () => {
    vi.mocked(getProjectAvatar).mockResolvedValueOnce({
      dataUrl: 'data:image/png;base64,saved-avatar',
      initialized: true,
    });
    const { container } = renderRow();

    await waitFor(() =>
      expect(container.querySelector('img')?.getAttribute('src')).toBe(
        'data:image/png;base64,saved-avatar',
      ),
    );

    fireEvent.contextMenu(screen.getByRole('button', { name: /^Open clip\.mp4/ }));
    fireEvent.click(screen.getByRole('menuitem', { name: 'Убрать аватарку' }));

    await waitFor(() => expect(container.querySelector('img')).toBeNull());
    expect(setProjectAvatar).toHaveBeenCalledWith(project.id, null);
  });

  it('pins and unpins a project from the context menu', async () => {
    renderRow();

    const openProject = screen.getByRole('button', { name: /^Open clip\.mp4/ });
    fireEvent.contextMenu(openProject);
    fireEvent.click(screen.getByRole('menuitem', { name: 'Закрепить' }));

    expect(getProjectPreferences(project.id).pinned).toBe(true);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        'set_project_pin_cmd',
        expect.objectContaining({ pinned: true }),
      ),
    );

    fireEvent.contextMenu(openProject);
    fireEvent.click(screen.getByRole('menuitem', { name: 'Открепить' }));

    await waitFor(() => expect(getProjectPreferences(project.id).pinned).toBe(false));
  });

  it.each([
    ['Shift+F10', { key: 'F10', shiftKey: true }],
    ['Menu key', { key: 'ContextMenu', code: 'ContextMenu' }],
  ])('opens the context menu with %s and focuses its first item', async (_name, key) => {
    renderRow();
    const openProject = screen.getByRole('button', { name: /^Open clip\.mp4/ });
    openProject.focus();

    fireEvent.keyDown(openProject, key);

    const firstItem = screen.getByRole('menuitem', { name: 'Переименовать' });
    await waitFor(() => expect(document.activeElement).toBe(firstItem));
  });

  it('navigates menu items with the keyboard and restores focus on Escape', async () => {
    renderRow();
    const openProject = screen.getByRole('button', { name: /^Open clip\.mp4/ });
    openProject.focus();
    fireEvent.keyDown(openProject, { key: 'F10', shiftKey: true });

    const rename = screen.getByRole('menuitem', { name: 'Переименовать' });
    const chooseAvatar = screen.getByRole('menuitem', { name: 'Выбрать аватарку' });
    await waitFor(() => expect(document.activeElement).toBe(rename));

    fireEvent.keyDown(rename, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(chooseAvatar);

    fireEvent.keyDown(chooseAvatar, { key: 'End' });
    expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'Удалить' }));

    fireEvent.keyDown(document.activeElement as HTMLElement, { key: 'Escape' });

    await waitFor(() => {
      expect(screen.queryByRole('menu')).toBeNull();
      expect(document.activeElement).toBe(openProject);
    });
  });

  it('rejects an unsupported avatar without saving it', async () => {
    const { container } = renderRow();
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;

    fireEvent.change(input, {
      target: { files: [new File(['<svg/>'], 'avatar.svg', { type: 'image/svg+xml' })] },
    });

    await waitFor(() => {
      expect(toast.warning).toHaveBeenCalled();
    });
    expect(setProjectAvatar).not.toHaveBeenCalled();
  });
});
