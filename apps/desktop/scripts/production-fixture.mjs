export function installFixture() {
  const timestamp = '2026-09-03T00:00:00Z';
  const project = {
    id: 'review-project',
    title: 'Review fixture',
    status: 'draft',
    source: null,
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
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener(_event, id) {
      window.__smokeCallbacks.delete(id);
    },
  };
  window.addEventListener('securitypolicyviolation', (event) =>
    window.__smokeViolations.push(event.effectiveDirective),
  );
  let callbackId = 0;
  window.__TAURI_INTERNALS__ = {
    transformCallback(callback) {
      window.__smokeCallbacks.set(++callbackId, callback);
      return callbackId;
    },
    unregisterCallback(id) {
      window.__smokeCallbacks.delete(id);
    },
    async invoke(command) {
      window.__smokeCommands.push(command);
      if (command === 'plugin:event|listen') return ++callbackId;
      if (command === 'plugin:event|unlisten') return null;
      if (command === 'list_projects_cmd') return [project];
      if (command === 'list_pending_youtube_imports_cmd') return [];
      if (command === 'list_jobs_cmd') return [job];
      if (command === 'get_project_cmd') return project;
      if (command === 'get_project_avatar_cmd') return { dataUrl: null, initialized: true };
      if (command === 'get_transcript_cmd') return null;
      if (command === 'list_youtube_subtitle_tracks_cmd') return [];
      window.__smokeUnknownCommands.push(command);
      throw new Error(`Unexpected smoke command: ${command}`);
    },
  };
}
