export function installUiLibraryFixture() {
  const base = window.__TAURI_INTERNALS__.invoke;
  let state = JSON.parse(sessionStorage.getItem('ui-library') ?? 'null') ?? {
    revision: 0,
    assets: [],
    folders: [],
    links: [],
    standaloneOwners: [],
    removedProjects: [],
    hiddenAssets: [],
    workspaces: [],
    browser: {
      scope: 'all',
      projectId: null,
      folderId: null,
      search: '',
      kind: '',
      selectedId: null,
    },
  };
  const persist = () => sessionStorage.setItem('ui-library', JSON.stringify(state));
  const hydrate = async () => {
    const projects = window.__e2e.projects;
    for (const project of projects.filter((project) => project.source)) {
      for (const kind of ['downloadedVideo', 'originalSubtitle']) {
        const artifacts = await base('list_project_artifacts_cmd', { projectId: project.id, kind });
        for (const artifact of artifacts.filter(
          (artifact) => artifact.kind === kind && artifact.state === 'ready',
        )) {
          if (state.assets.some((asset) => asset.artifactId === artifact.id)) continue;
          state.assets.push({
            id: artifact.id,
            ownerProjectId: project.id,
            artifactId: artifact.id,
            name: `${project.title} · ${kind === 'originalSubtitle' ? 'OriginalSubtitle' : 'DownloadedVideo'}`,
            kind: kind === 'originalSubtitle' ? 'subtitle' : 'video',
            format: kind === 'originalSubtitle' ? 'srt' : null,
            text: null,
            parentId: null,
            tags: [],
            openedAt: null,
          });
          state.links.push({ assetId: artifact.id, projectId: project.id, folderId: null });
          state.revision++;
        }
      }
    }
    persist();
    return structuredClone(state);
  };
  window.__TAURI_INTERNALS__.invoke = async (command, args = {}) => {
    if (command === 'get_library_cmd') return hydrate();
    if (command === 'save_library_cmd') {
      if (args.state.revision !== state.revision)
        throw { code: 'CONFLICT', message: 'Stale fixture library revision' };
      state = { ...structuredClone(args.state), revision: state.revision + 1 };
      persist();
      return structuredClone(state);
    }
    if (command === 'validate_media_project_cmd') return null;
    if (command === 'get_tool_context_cmd') {
      const asset = state.assets.find((asset) => asset.id === args.assetId);
      if (!asset) throw { code: 'NOT_FOUND', message: 'Fixture material not found' };
      return {
        projectId: args.projectId,
        assetId: asset.id,
        artifactId: asset.artifactId,
        version: null,
        trackId: null,
      };
    }
    if (command === 'get_media_project_cmd')
      return { version: 1, project_id: args.projectId, assets: [], results: [] };
    return base(command, args);
  };
}
