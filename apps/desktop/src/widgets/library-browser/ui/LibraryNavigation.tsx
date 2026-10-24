import { useState } from 'react';
import { useLibrary, useStudioActions, kindNames } from '@/entities/library';
import type { LibraryBrowser } from '@/shared/api/contracts/library';
import { button } from './controls';
type Props = {
  browser: LibraryBrowser;
  navigate: (patch: Partial<LibraryBrowser>) => void;
  create: (kind: 'project' | 'folder') => void;
};
export function LibraryNavigation({ browser, navigate }: Props) {
  const { state, projects, openProject } = useLibrary();
  const actions = useStudioActions();
  const [over, setOver] = useState<string | null>(null);
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
          onClick={() => navigate({ scope, projectId: null, folderId: null, selectedId: null })}
        >
          {label}
        </button>
      ))}
      <div className="mb-2 mt-7 flex items-center justify-between">
        <span className="text-xs uppercase text-muted">Проекты</span>
        <button
          className={button}
          aria-label="Создать проект"
          onClick={() => actions.run('new-project')}
        >
          +
        </button>
      </div>
      {projects
        .filter(
          (p) => !state.standaloneOwners.includes(p.id) && !state.removedProjects.includes(p.id),
        )
        .map((project) => (
          <div key={project.id} className="mb-2">
            <button
              className={`${button} w-full text-left ${over === project.id ? 'ring-2 ring-focus' : ''} ${browser.projectId === project.id ? 'bg-surface-active' : ''}`}
              onClick={() =>
                navigate({
                  scope: 'project',
                  projectId: project.id,
                  folderId: null,
                  selectedId: null,
                  kind: '',
                })
              }
              onDoubleClick={() => void openProject(project.id).catch(() => {})}
              title="Двойной щелчок или Enter — открыть проект"
              onContextMenu={(e) => {
                navigate({
                  scope: 'project',
                  projectId: project.id,
                  folderId: null,
                  selectedId: null,
                });
                actions.context(e, { projectId: project.id, assetId: null });
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  void openProject(project.id).catch(() => {});
                } else actions.context(e, { projectId: project.id, assetId: null });
              }}
              onDragOver={(e) => {
                if (e.dataTransfer.types.includes('application/x-auralis-asset')) {
                  e.preventDefault();
                  e.dataTransfer.dropEffect = 'copy';
                  setOver(project.id);
                }
              }}
              onDragLeave={() => setOver(null)}
              onDrop={(e) => {
                e.preventDefault();
                setOver(null);
                const id = e.dataTransfer.getData('application/x-auralis-asset');
                if (state.assets.some((a) => a.id === id)) actions.drop(id, project.id);
              }}
            >
              {project.title}
            </button>
            {browser.projectId === project.id && (
              <div className="ml-3 mt-1 space-y-1 border-l border-border pl-2">
                {Object.entries(kindNames).map(([kind, name]) => (
                  <button
                    key={kind}
                    className={`w-full rounded px-2 py-1 text-left text-xs hover:bg-surface-raised ${browser.kind === kind ? 'bg-surface-active' : ''}`}
                    onClick={() =>
                      navigate({
                        scope: 'project',
                        projectId: project.id,
                        folderId: null,
                        kind,
                        selectedId: null,
                      })
                    }
                  >
                    {name}
                  </button>
                ))}
              </div>
            )}
          </div>
        ))}
    </nav>
  );
}
