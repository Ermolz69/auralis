import { useState } from 'react';
import { LibraryContext, type LibraryContextValue } from '@/entities/library';
import { emptyLibrary } from '@/shared/api/contracts/library';
import { StudioShell } from './StudioShell';
import { JobContext, type JobStoreState } from '@/entities/job';
const activeJobs: JobStoreState = {
  phase: 'ready',
  buffer: [],
  pendingRefetch: false,
  generation: 0,
  jobs: Object.fromEntries(
    ['running', 'pending', 'cancelling'].map((status, index) => [
      `job-${index}`,
      {
        id: `job-${index}`,
        projectId: null,
        kind: 'speech',
        revision: 1,
        title: ['Подготовка субтитров', 'Синтез речи · Глава 2', 'Перевод · Глава 3'][index],
        status: status as 'running' | 'pending' | 'cancelling',
        stage: null,
        progress: {
          percent: [42, 0, 73][index],
          message: '',
          currentStep: null,
          processedItems: null,
          totalItems: null,
        },
        error: null,
        createdAt: '2026-10-24T10:00:00Z',
        updatedAt: '2026-10-24T10:00:00Z',
      },
    ]),
  ),
};
const projectId = '10000000-0000-4000-8000-000000000001';
export function StudioStory({
  project = false,
  empty = false,
  busy = false,
}: {
  project?: boolean;
  empty?: boolean;
  busy?: boolean;
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
    if (project && !empty) {
      state.folders.push({ id: 'folder-1', projectId, parentId: null, name: 'Глава первая' });
      state.links.push({ projectId, assetId: state.assets[0].id, folderId: 'folder-1' });
    }
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
      <JobContext.Provider value={busy ? activeJobs : null}>
        <StudioShell />
      </JobContext.Provider>
    </LibraryContext.Provider>
  );
}
