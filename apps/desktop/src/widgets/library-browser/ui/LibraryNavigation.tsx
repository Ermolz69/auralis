import { useLibrary } from '@/entities/library';
import type { LibraryBrowser } from '@/shared/api/contracts/library';
import { button } from './controls';
type Props = {
  browser: LibraryBrowser;
  navigate: (patch: Partial<LibraryBrowser>) => void;
  create: (kind: 'project' | 'folder') => void;
};
export function LibraryNavigation({ browser, navigate, create }: Props) {
  const { state, projects, mutate, openProject } = useLibrary();
  const projectList = projects.filter(
    (p) => !state.standaloneOwners.includes(p.id) && !state.removedProjects.includes(p.id),
  );
  const drop = (event: React.DragEvent, projectId: string, folderId: string | null) => {
    event.preventDefault();
    const assetId = event.dataTransfer.getData('application/x-auralis-asset');
    if (!state.assets.some((a) => a.id === assetId)) return;
    void mutate((draft) => {
      const existing = draft.links.find((l) => l.projectId === projectId && l.assetId === assetId);
      if (existing) existing.folderId = folderId;
      else draft.links.push({ assetId, projectId, folderId });
    }).catch(() => {});
  };
  const folders = (projectId: string, parentId: string | null, depth = 0): React.ReactNode =>
    state.folders
      .filter((f) => f.projectId === projectId && f.parentId === parentId)
      .map((folder) => (
        <div key={folder.id} style={{ paddingLeft: 12 }}>
          <button
            className={`${button} w-full text-left ${browser.folderId === folder.id ? 'bg-surface-active' : ''}`}
            onClick={() => navigate({ scope: 'project', projectId, folderId: folder.id })}
            onDragOver={(e) => e.preventDefault()}
            onDrop={(e) => drop(e, projectId, folder.id)}
          >
            {folder.name}
          </button>
          {depth < 50 && folders(projectId, folder.id, depth + 1)}
        </div>
      ));
  return (
    <nav
      aria-label="Библиотека и проекты"
      className="h-full overflow-auto border-r border-border bg-surface p-3"
    >
      <p className="mb-3 text-xs uppercase text-muted">Библиотека</p>
      {(
        [
          ['all', 'Все материалы'],
          ['unassigned', 'Без проекта'],
          ['recent', 'Недавние'],
        ] as const
      ).map(([scope, label]) => (
        <button
          key={scope}
          className={`${button} mb-1 w-full text-left ${browser.scope === scope ? 'bg-surface-active' : ''}`}
          aria-current={browser.scope === scope ? 'page' : undefined}
          onClick={() => navigate({ scope, projectId: null, folderId: null })}
        >
          {label}
        </button>
      ))}
      <div className="mb-2 mt-7 flex items-center justify-between">
        <span className="text-xs uppercase text-muted">Проекты</span>
        <button className={button} aria-label="Создать проект" onClick={() => create('project')}>
          +
        </button>
      </div>
      {projectList.map((project) => (
        <div key={project.id} className="mb-2">
          <button
            className={`${button} w-full text-left ${browser.projectId === project.id && !browser.folderId ? 'bg-surface-active' : ''}`}
            onClick={() => navigate({ scope: 'project', projectId: project.id, folderId: null })}
            onDoubleClick={() => void openProject(project.id).catch(() => {})}
            title="Двойной щелчок или Enter — открыть проект"
            onKeyDown={(event) => {
              if (event.key !== 'Enter') return;
              event.preventDefault();
              void openProject(project.id).catch(() => {});
            }}
            onDragOver={(e) => e.preventDefault()}
            onDrop={(e) => drop(e, project.id, null)}
          >
            {project.title}
          </button>
          {folders(project.id, null)}
        </div>
      ))}
      {browser.projectId && (
        <button className={`${button} mt-2 w-full`} onClick={() => create('folder')}>
          + Папка
        </button>
      )}
    </nav>
  );
}
