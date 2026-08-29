// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { getProjectPreferences, updateProjectPreferences, type Project } from '@/entities/project';
import { toast } from '@/shared/ui/toast';
import { ProjectListRow } from './ProjectListRow';

vi.mock('@/shared/ui/toast', () => ({
  toast: { error: vi.fn() },
}));

const project: Project = {
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
});

afterEach(() => cleanup());

function renderRow() {
  return render(
    <ProjectListRow
      project={project}
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

  it('loads a saved avatar and lets the user remove it', () => {
    updateProjectPreferences(project.id, { avatar: 'data:image/png;base64,saved-avatar' });
    const { container } = renderRow();

    expect(container.querySelector('img')?.getAttribute('src')).toBe(
      'data:image/png;base64,saved-avatar',
    );

    fireEvent.contextMenu(screen.getByRole('button', { name: /^Open clip\.mp4/ }));
    fireEvent.click(screen.getByRole('menuitem', { name: 'Убрать аватарку' }));

    expect(container.querySelector('img')).toBeNull();
    expect(getProjectPreferences(project.id).avatar).toBeNull();
  });

  it('pins and unpins a project from the context menu', () => {
    renderRow();

    const openProject = screen.getByRole('button', { name: /^Open clip\.mp4/ });
    fireEvent.contextMenu(openProject);
    fireEvent.click(screen.getByRole('menuitem', { name: 'Закрепить' }));

    expect(getProjectPreferences(project.id).pinned).toBe(true);

    fireEvent.contextMenu(openProject);
    fireEvent.click(screen.getByRole('menuitem', { name: 'Открепить' }));

    expect(getProjectPreferences(project.id).pinned).toBe(false);
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
      expect(toast.error).toHaveBeenCalledWith('Поддерживаются только PNG, JPEG, WebP и GIF');
    });
    expect(getProjectPreferences(project.id).avatar).toBeNull();
  });
});
