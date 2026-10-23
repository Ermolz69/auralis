import { describeCommandError } from '@/shared/api/contracts/error';
import { useState } from 'react';
import { useLibrary } from '@/entities/library';
import { renameProject } from '@/entities/project';
import type { LibraryBrowser } from '@/shared/api/contracts/library';
import { button, field } from './controls';
export function LibraryProjectActions({
  browser,
  navigate,
}: {
  browser: LibraryBrowser;
  navigate: (value: Partial<LibraryBrowser>) => void;
}) {
  const { state, projects, mutate, refresh, openProject } = useLibrary();
  const project = projects.find((p) => p.id === browser.projectId);
  const folder = state.folders.find((f) => f.id === browser.folderId);
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState('');
  const [confirm, setConfirm] = useState(false);
  const [error, setError] = useState('');
  if (!project) return null;
  const rename = async () => {
    try {
      if (folder)
        await mutate((draft) => {
          draft.folders.find((f) => f.id === folder.id)!.name = name.trim();
        });
      else {
        await renameProject(project.id, name.trim(), project.revision);
        await refresh();
      }
      setEditing(false);
    } catch (e) {
      setError(describeCommandError(e));
    }
  };
  const remove = async () => {
    try {
      await mutate((draft) => {
        if (folder) {
          for (const f of draft.folders) if (f.parentId === folder.id) f.parentId = folder.parentId;
          for (const link of draft.links)
            if (link.folderId === folder.id) link.folderId = folder.parentId;
          draft.folders = draft.folders.filter((f) => f.id !== folder.id);
        } else {
          draft.removedProjects.push(project.id);
          draft.links = draft.links.filter((l) => l.projectId !== project.id);
          draft.folders = draft.folders.filter((f) => f.projectId !== project.id);
          draft.workspaces = draft.workspaces.filter((w) => w.projectId !== project.id);
        }
      });
      setConfirm(false);
      navigate(
        folder ? { folderId: folder.parentId } : { scope: 'all', projectId: null, folderId: null },
      );
    } catch (e) {
      setError(describeCommandError(e));
    }
  };
  return (
    <div className="mb-4 flex flex-wrap items-center gap-2 text-xs">
      <button className={button} onClick={() => void openProject(project.id).catch(() => {})}>
        Открыть рабочее пространство
      </button>
      <button
        className={button}
        onClick={() => {
          setName(folder?.name ?? project.title);
          setEditing(true);
        }}
      >
        Переименовать {folder ? 'папку' : 'проект'}
      </button>
      <button className={button} onClick={() => setConfirm(true)}>
        Удалить {folder ? 'папку' : 'проект'}…
      </button>
      {editing && (
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            void rename();
          }}
        >
          <input
            aria-label="Новое название"
            className={field}
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            maxLength={200}
          />
          <button className={button}>Сохранить</button>
          <button type="button" className={button} onClick={() => setEditing(false)}>
            Отмена
          </button>
        </form>
      )}
      {confirm && (
        <div
          role="alertdialog"
          aria-label="Удаление раздела"
          className="w-full border border-border p-3"
        >
          <p>Материалы останутся в библиотеке. Содержимое папки перейдёт в родительскую.</p>
          <button className={button} onClick={() => void remove()}>
            Подтвердить удаление раздела
          </button>
          <button className={button} onClick={() => setConfirm(false)}>
            Отмена
          </button>
        </div>
      )}
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
