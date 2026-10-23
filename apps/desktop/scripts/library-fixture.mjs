export function installLibraryFixture() {
  const base = window.__TAURI_INTERNALS__.invoke;
  const alpha = '10000000-0000-4000-8000-000000000001';
  const beta = '10000000-0000-4000-8000-000000000002';
  const owner = '10000000-0000-4000-8000-000000000003';
  const asset = '20000000-0000-4000-8000-000000000001';
  const shared = '20000000-0000-4000-8000-000000000002';
  const timestamp = '2026-10-23T10:00:00Z';
  const project = (id, title) => ({
    id,
    title,
    revision: 1,
    status: 'draft',
    source: null,
    metadata: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  });
  let projects = JSON.parse(sessionStorage.getItem('library-projects') || 'null') ?? [
    project(alpha, 'Серия 01'),
    project(beta, 'Серия 02'),
    project(owner, 'Контекст текста'),
  ];
  let state = JSON.parse(sessionStorage.getItem('library-state') || 'null') ?? {
    revision: 0,
    assets: [
      {
        id: asset,
        ownerProjectId: owner,
        name: 'Самостоятельный текст',
        kind: 'text',
        text: 'Расстояние 12 км.',
        artifactId: null,
        format: 'txt',
        parentId: null,
        tags: ['проба'],
        openedAt: null,
      },
      {
        id: shared,
        ownerProjectId: owner,
        name: 'Общий рассказ',
        kind: 'text',
        text: 'Добрый вечер.',
        artifactId: null,
        format: 'txt',
        parentId: null,
        tags: [],
        openedAt: null,
      },
    ],
    folders: [],
    links: [{ assetId: shared, projectId: alpha, folderId: null }],
    standaloneOwners: [owner],
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
  const persist = () => {
    sessionStorage.setItem('library-state', JSON.stringify(state));
    sessionStorage.setItem('library-projects', JSON.stringify(projects));
  };
  window.__libraryIds = { alpha, beta, owner, asset, shared };
  window.__librarySaves = [];
  window.__libraryRuntimeMissing = false;
  window.__TAURI_INTERNALS__.invoke = async (command, args) => {
    if (command === 'get_library_cmd') return structuredClone(state);
    if (command === 'save_library_cmd') {
      if (window.__libraryConflict) {
        window.__libraryConflict = false;
        state.revision++;
        persist();
        throw { code: 'CONFLICT', message: 'Concurrent library change' };
      }
      if (args.state.revision !== state.revision)
        throw { code: 'CONFLICT', message: 'Stale revision' };
      state = { ...structuredClone(args.state), revision: state.revision + 1 };
      persist();
      window.__librarySaves.push(structuredClone(state));
      return state;
    }
    if (command === 'list_projects_cmd') return projects;
    if (command === 'get_project_cmd')
      return projects.find((p) => p.id === args.projectId) ?? projects[0];
    if (command === 'create_project_cmd') {
      if (window.__libraryRuntimeMissing)
        throw { code: 'VALIDATION', message: 'PUZA CONTRACT runtime is unavailable' };
      const item = project(crypto.randomUUID(), args.title);
      projects.push(item);
      persist();
      return item;
    }
    if (command === 'rename_project_cmd') {
      const item = projects.find((p) => p.id === args.projectId);
      item.title = args.title;
      item.revision++;
      persist();
      return item;
    }
    if (command === 'delete_project_cmd') {
      projects = projects.filter((p) => p.id !== args.projectId);
      persist();
      return null;
    }
    if (command === 'list_project_artifacts_cmd') return [];
    if (command === 'list_project_translations_cmd') return [];
    return base(command, args);
  };
}
