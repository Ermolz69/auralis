export function installFixture() {
  const timestamp = '2026-09-03T00:00:00Z';
  const project = {
    id: 'review-project',
    title: 'Review fixture',
    status: 'draft',
    source: null,
    revision: 1,
    metadata: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  const job = {
    id: 'foreign-job',
    projectId: 'another-project',
    kind: 'dubbing',
    title: 'Background fixture job',
    status: 'running',
    stage: 'importYoutubeSubtitles',
    revision: 1,
    progress: {
      percent: 25,
      message: 'Working',
      currentStep: null,
      processedItems: null,
      totalItems: null,
    },
    error: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  window.__smokeViolations = [];
  window.__smokeCommands = [];
  window.__smokeUnknownCommands = [];
  window.__smokeCallbacks = new Map();
  window.__smokeListeners = new Map();
  window.__smokeJob = job;
  window.isTauri = true;
  window.__smokeWindow = { maximized: false, minimized: false, closed: false };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener(_event, id) {
      window.__smokeCallbacks.delete(id);
    },
  };
  window.addEventListener('securitypolicyviolation', (event) =>
    window.__smokeViolations.push(event.effectiveDirective),
  );
  let callbackId = 0;
  let library = JSON.parse(sessionStorage.getItem('smoke-library') || 'null') ?? {
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
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
    transformCallback(callback) {
      window.__smokeCallbacks.set(++callbackId, callback);
      return callbackId;
    },
    unregisterCallback(id) {
      window.__smokeCallbacks.delete(id);
    },
    async invoke(command, args) {
      window.__smokeCommands.push(command);
      if (command === 'plugin:app|version') return '0.1.0';
      if (command === 'plugin:updater|check') return null;
      if (command === 'plugin:window|is_maximized') return window.__smokeWindow.maximized;
      if (command === 'plugin:window|toggle_maximize') {
        window.__smokeWindow.maximized = !window.__smokeWindow.maximized;
        return null;
      }
      if (command === 'plugin:window|minimize') {
        if (window.__smokeWindowFailure) throw new Error('Window action rejected');
        window.__smokeWindow.minimized = true;
        return null;
      }
      if (command === 'plugin:window|close') {
        window.__smokeWindow.closed = true;
        return null;
      }
      if (command === 'plugin:event|listen') {
        window.__smokeListeners.set(args.event, args.handler);
        return ++callbackId;
      }
      if (command === 'plugin:event|unlisten') return null;
      if (command === 'get_library_cmd') return structuredClone(library);
      if (command === 'save_library_cmd') {
        library = { ...structuredClone(args.state), revision: library.revision + 1 };
        sessionStorage.setItem('smoke-library', JSON.stringify(library));
        return library;
      }
      if (command === 'list_project_artifacts_cmd') return [];
      if (command === 'list_projects_cmd') return [project];
      if (command === 'list_pending_youtube_imports_cmd') return [];
      if (command === 'list_artifact_recovery_cmd') return [];
      if (command === 'get_color_theme_cmd') return { value: 'dark', revision: 1 };
      if (command === 'get_project_pins_cmd') return { entries: [], migrated: true };
      if (command === 'list_jobs_cmd') return [job];
      if (command === 'list_job_history_page_cmd') return { jobs: [], nextCursor: null };
      if (command === 'get_project_cmd') return project;
      if (command === 'get_project_avatar_cmd') return { dataUrl: null, initialized: true };
      if (command === 'get_transcript_cmd') return null;
      if (command === 'list_youtube_subtitle_tracks_cmd') return [];
      window.__smokeUnknownCommands.push(command);
      throw new Error(`Unexpected smoke command: ${command}`);
    },
  };
}
