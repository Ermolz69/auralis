import { useState } from 'react';
import { LibraryContext, type LibraryContextValue } from '@/entities/library';
import { emptyLibrary } from '@/shared/api/contracts/library';
import { StudioShell } from './StudioShell';
const projectId = '10000000-0000-4000-8000-000000000001';
export function StudioStory({
  project = false,
  empty = false,
}: {
  project?: boolean;
  empty?: boolean;
}) {
  const [state, setState] = useState(() => {
    const state = emptyLibrary();
    if (!empty)
      state.assets.push({
        id: '20000000-0000-4000-8000-000000000001',
        ownerProjectId: projectId,
        name: 'Вступление — русский текст',
        kind: 'text',
        artifactId: null,
        format: 'txt',
        text: 'Текст можно открыть без проекта и использовать нужные инструменты.',
        parentId: null,
        tags: ['рассказчик', 'черновик'],
        openedAt: null,
      });
    if (project)
      state.workspaces.push({
        id: `project:${projectId}`,
        projectId,
        assetId: null,
        activeTab: null,
        tabs: [],
      });
    return state;
  });
  const [view, setView] = useState<LibraryContextValue['view']>(project ? 'workspace' : 'library');
  const projects = [
    {
      id: projectId,
      revision: 1,
      title: 'Рассказ · глава первая',
      status: 'draft' as const,
      source: null,
      metadata: null,
      createdAt: '2026-10-23T10:00:00Z',
      updatedAt: '2026-10-23T10:00:00Z',
    },
  ];
  return (
    <LibraryContext.Provider
      value={{
        state,
        projects,
        loading: false,
        error: null,
        view,
        setView,
        workspaceId: project ? `project:${projectId}` : null,
        activateWorkspace: () => setView('workspace'),
        mutate: async (change) =>
          setState((current) => {
            const draft = structuredClone(current);
            change(draft);
            return draft;
          }),
        refresh: async () => {},
        openProject: async () => {},
        openAsset: async () => {},
        openTool: async () => {},
      }}
    >
      <StudioShell />
    </LibraryContext.Provider>
  );
}
