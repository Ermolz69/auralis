import type { LibraryState } from '@/shared/api/contracts/library';
import { emptyLibrary } from '@/shared/api/contracts/library';
import type { Project } from '@/shared/api/contracts/project';

export const studioProject: Project = {
  id: '10000000-0000-4000-8000-000000000001',
  revision: 1,
  title: 'Рассказ · глава первая',
  status: 'draft',
  source: null,
  metadata: null,
  createdAt: '2026-10-23T10:00:00Z',
  updatedAt: '2026-10-23T10:00:00Z',
};

export function studioBoundary({ empty = false, project = false, failRefresh = false } = {}) {
  let state = emptyLibrary();
  if (!empty) {
    state.assets.push({
      id: '20000000-0000-4000-8000-000000000001',
      ownerProjectId: studioProject.id,
      name: 'Вступление — русский текст',
      kind: 'text',
      artifactId: null,
      format: 'txt',
      text: 'Текст можно открыть без проекта и использовать нужные инструменты.',
      parentId: null,
      tags: ['рассказчик', 'черновик'],
      openedAt: null,
    });
  }
  if (project && !empty) {
    state.folders.push({
      id: 'folder-1',
      projectId: studioProject.id,
      parentId: null,
      name: 'Глава первая',
    });
    state.links.push({
      projectId: studioProject.id,
      assetId: state.assets[0].id,
      folderId: 'folder-1',
    });
  }
  const internal = (
    window as unknown as {
      __TAURI_INTERNALS__: { invoke: (command: string, args?: unknown) => Promise<unknown> };
    }
  ).__TAURI_INTERNALS__;
  const previous = internal.invoke;
  let refreshes = 0;
  let contextFailure = false;
  internal.invoke = async (command, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    switch (command) {
      case 'get_library_cmd':
        if (failRefresh && refreshes++ === 0)
          throw { code: 'IO', message: 'Library index is temporarily unavailable' };
        return structuredClone(state);
      case 'list_projects_cmd':
        return [structuredClone(studioProject)];
      case 'save_library_cmd':
        state = structuredClone(args.state as LibraryState);
        state.revision++;
        return structuredClone(state);
      case 'get_project_cmd':
        if (args.projectId !== studioProject.id) throw new Error('Unexpected project identity');
        return structuredClone(studioProject);
      case 'validate_media_project_cmd':
        return null;
      case 'get_tool_context_cmd':
        if (contextFailure)
          throw { code: 'IO', message: 'Material context is temporarily unavailable' };
        if (
          args.projectId !== studioProject.id ||
          !state.assets.some((asset) => asset.id === args.assetId)
        )
          throw new Error('Unexpected material identity');
        return {
          projectId: args.projectId,
          assetId: args.assetId,
          artifactId: null,
          version: 'version-one',
          trackId: null,
        };
      case 'get_speech_catalog_cmd':
        return { profiles: [], voices: [] };
      case 'get_speech_voice_selection_cmd':
        return null;
      case 'list_project_artifacts_cmd':
      case 'list_project_translations_cmd':
      case 'list_speech_runs_cmd':
      case 'list_audio_assemblies_cmd':
        return [];
      case 'get_media_project_cmd':
        return { results: [] };
      default:
        return previous(command, payload);
    }
  };
  return {
    snapshot: () => structuredClone(state),
    failContext: (value: boolean) => {
      contextFailure = value;
    },
    restore: () => {
      internal.invoke = previous;
    },
  };
}
