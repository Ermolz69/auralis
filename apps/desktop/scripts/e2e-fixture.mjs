export function installE2EFixture(seed) {
  const copy = (value) => (value === undefined ? undefined : structuredClone(value));
  const timestamp = '2026-09-03T12:00:00Z';
  const input = copy(seed ?? {});
  const failures = Object.fromEntries(
    Object.entries(input.failures ?? {}).map(([command, failure]) => [
      command,
      { remaining: failure.times ?? 1, error: failure.error },
    ]),
  );
  const state = {
    projects: input.projects ?? [],
    jobs: input.jobs ?? [],
    pendingImports: input.pendingImports ?? [],
    tracks: input.tracks ?? {},
    transcripts: input.transcripts ?? {},
    avatars: input.avatars ?? {},
    selectedFile: input.selectedFile ?? null,
    calls: [],
    unknownCommands: [],
    openedFolders: [],
    updater: input.updater ?? null,
    restarted: false,
    failures,
    nextProject: 1,
    nextJob: 1,
  };

  const mediaMetadata = {
    durationMs: 185_000,
    width: 1920,
    height: 1080,
    fps: 30,
    videoCodec: 'h264',
    audioCodec: 'aac',
    sampleRate: 48_000,
    audioChannels: 2,
    container: 'mp4',
    hasVideo: true,
    hasAudio: true,
    audioTracks: [
      {
        streamIndex: 1,
        codec: 'aac',
        channels: 2,
        sampleRate: 48_000,
        language: 'en',
        isDefault: true,
      },
    ],
    streams: [
      { index: 0, codecType: 'video', codecName: 'h264', durationMs: 185_000 },
      {
        index: 1,
        codecType: 'audio',
        codecName: 'aac',
        language: 'en',
        durationMs: 185_000,
      },
    ],
  };

  const findProject = (projectId) => state.projects.find((project) => project.id === projectId);
  const replaceProject = (project) => {
    const index = state.projects.findIndex((item) => item.id === project.id);
    if (index >= 0) state.projects[index] = project;
    else state.projects.push(project);
    return project;
  };
  const createDraft = (title, id = `created-${state.nextProject++}`) => ({
    id,
    title,
    status: 'draft',
    source: null,
    metadata: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  });
  const createJob = (projectId, title = 'Subtitle import') => ({
    id: `created-job-${state.nextJob++}`,
    kind: 'dubbing',
    revision: 1,
    projectId,
    title,
    status: 'running',
    stage: 'extractOrGenerateTranscript',
    progress: {
      percent: 10,
      message: 'Starting subtitle import',
      currentStep: 'extractOrGenerateTranscript',
      processedItems: null,
      totalItems: null,
    },
    error: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  });
  const failIfConfigured = (command) => {
    const configured = state.failures[command];
    if (!configured || configured.remaining <= 0) return;
    configured.remaining -= 1;
    throw copy(configured.error);
  };

  window.__e2e = state;
  window.__e2eCallbacks = new Map();
  window.isTauri = true;
  let callbackId = 0;
  let eventId = 0;

  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener(_event, id) {
      window.__e2eCallbacks.delete(id);
    },
  };

  window.__TAURI_INTERNALS__ = {
    transformCallback(callback) {
      const id = ++callbackId;
      window.__e2eCallbacks.set(id, callback);
      return id;
    },
    unregisterCallback(id) {
      window.__e2eCallbacks.delete(id);
    },
    async invoke(command, args = {}) {
      state.calls.push({ command, args: copy(args) });
      failIfConfigured(command);

      if (command === 'plugin:event|listen') return ++eventId;
      if (command === 'plugin:event|unlisten') return null;
      if (command === 'plugin:app|version') {
        return state.updater?.currentVersion ?? '0.1.0';
      }
      if (command === 'plugin:updater|check') {
        const available = state.updater?.available;
        return available
          ? copy({
              rid: 81,
              currentVersion: state.updater.currentVersion,
              version: available.version,
              date: available.date,
              body: available.body,
              rawJson: {},
            })
          : null;
      }
      if (command === 'plugin:updater|download_and_install') {
        const callback = window.__e2eCallbacks.get(args.onEvent.id);
        callback?.({ index: 0, message: { event: 'Started', data: { contentLength: 100 } } });
        callback?.({ index: 1, message: { event: 'Progress', data: { chunkLength: 100 } } });
        callback?.({ index: 2, message: { event: 'Finished' } });
        callback?.({ index: 3, end: true });
        return null;
      }
      if (command === 'plugin:process|restart') {
        state.restarted = true;
        return null;
      }
      if (command === 'plugin:resources|close') return null;
      if (command === 'plugin:dialog|open') return state.selectedFile;
      if (command === 'health_check') return 'healthy';
      if (command === 'list_projects_cmd') return copy(state.projects);
      if (command === 'list_pending_youtube_imports_cmd') return copy(state.pendingImports);
      if (command === 'list_jobs_cmd') return copy(state.jobs);
      if (command === 'list_jobs_snapshot_cmd') {
        return copy(state.jobs.filter((job) => job.projectId === args.projectId));
      }
      if (command === 'get_project_cmd') {
        const project = findProject(args.projectId);
        if (!project) throw { code: 'NOT_FOUND', message: 'Project not found' };
        return copy(project);
      }
      if (command === 'get_project_avatar_cmd') {
        return copy(state.avatars[args.projectId] ?? { dataUrl: null, initialized: true });
      }
      if (command === 'set_project_avatar_cmd') {
        const avatar = { dataUrl: args.dataUrl, initialized: true };
        state.avatars[args.projectId] = avatar;
        return copy(avatar);
      }
      if (command === 'get_transcript_cmd') {
        return copy(state.transcripts[args.projectId] ?? null);
      }
      if (command === 'list_youtube_subtitle_tracks_cmd') {
        return copy(state.tracks[args.projectId] ?? []);
      }
      if (command === 'create_project_cmd') {
        return copy(replaceProject(createDraft(args.title)));
      }
      if (command === 'create_project_from_youtube_cmd') {
        const current = args.projectId ? findProject(args.projectId) : null;
        const project = replaceProject({
          ...(current ?? createDraft('YouTube project', args.projectId)),
          source: { kind: 'youtubeUrl', url: args.url },
          metadata: copy(mediaMetadata),
          status: 'ready_for_processing',
          updatedAt: timestamp,
        });
        return copy(project);
      }
      if (command === 'probe_local_media_cmd') return copy(mediaMetadata);
      if (command === 'import_local_media_cmd') {
        const current = findProject(args.projectId);
        if (!current) throw { code: 'NOT_FOUND', message: 'Project not found' };
        const filename = args.path.split(/[/\\]/).filter(Boolean).at(-1) ?? 'video.mp4';
        const project = replaceProject({
          ...current,
          source: {
            kind: 'managedLocalFile',
            artifactId: 'local-artifact',
            originalFilename: filename,
          },
          metadata: copy(mediaMetadata),
          status: 'ready_for_processing',
          updatedAt: timestamp,
        });
        return copy(project);
      }
      if (command === 'rename_project_cmd') {
        const current = findProject(args.projectId);
        if (!current) throw { code: 'NOT_FOUND', message: 'Project not found' };
        return copy(replaceProject({ ...current, title: args.title, updatedAt: timestamp }));
      }
      if (command === 'open_project_folder_cmd') {
        state.openedFolders.push(args.projectId);
        return null;
      }
      if (command === 'delete_project_cmd') {
        state.projects = state.projects.filter((project) => project.id !== args.projectId);
        return null;
      }
      if (command === 'resume_youtube_import_cmd') {
        const pending = state.pendingImports.find((item) => item.projectId === args.projectId);
        state.pendingImports = state.pendingImports.filter(
          (item) => item.projectId !== args.projectId,
        );
        const current =
          findProject(args.projectId) ??
          createDraft(pending?.title ?? 'Recovered import', args.projectId);
        const project = replaceProject({
          ...current,
          source: { kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=recovered' },
          metadata: copy(mediaMetadata),
          status: 'ready_for_processing',
          updatedAt: timestamp,
        });
        return copy(project);
      }
      if (command === 'discard_youtube_import_cmd') {
        state.pendingImports = state.pendingImports.filter(
          (item) => item.projectId !== args.projectId,
        );
        return null;
      }
      if (command === 'start_project_mock_pipeline_cmd') {
        const current = findProject(args.projectId);
        if (!current) throw { code: 'NOT_FOUND', message: 'Project not found' };
        const project = replaceProject({ ...current, status: 'processing', updatedAt: timestamp });
        const job = createJob(project.id);
        state.jobs.unshift(job);
        return copy({ project, job });
      }
      if (command === 'cancel_job_cmd') {
        const index = state.jobs.findIndex((job) => job.id === args.jobId);
        if (index < 0) throw { code: 'NOT_FOUND', message: 'Job not found' };
        const job = {
          ...state.jobs[index],
          revision: state.jobs[index].revision + 1,
          status: 'cancelled',
          updatedAt: timestamp,
        };
        state.jobs[index] = job;
        return copy(job);
      }

      state.unknownCommands.push(command);
      throw new Error(`Unexpected E2E command: ${command}`);
    },
  };
}
