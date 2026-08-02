import { mockIPC } from '@tauri-apps/api/mocks';
import type { CommandMap } from '../src/shared/api/contracts/commandMap';
import type { Job } from '../src/shared/api/contracts/job';
import type { MediaMetadata } from '../src/shared/api/contracts/media';
import type { CreateProjectResponse, Project } from '../src/shared/api/contracts/project';
import type { Transcript } from '../src/shared/api/contracts/transcript';

const createdAt = '2026-08-02T00:00:00.000Z';
const updatedAt = '2026-08-02T00:00:01.000Z';

const storyTranscripts: Record<string, Transcript> = {
  'youtube-ready': {
    language: 'en',
    segments: [
      {
        id: 'segment-1',
        index: 0,
        startMs: 1000,
        endMs: 4200,
        sourceText: 'Welcome to the product walkthrough.',
      },
      {
        id: 'segment-2',
        index: 1,
        startMs: 4400,
        endMs: 7800,
        sourceText: 'The transcript is available as read-only text.',
      },
    ],
  },
  'local-unavailable': {
    language: 'en',
    segments: [],
  },
};

const storyMediaMetadata: MediaMetadata = {
  durationMs: 184000,
  width: 1920,
  height: 1080,
  fps: 29.97,
  videoCodec: 'h264',
  container: 'mp4',
  hasVideo: true,
  hasAudio: true,
  audioTracks: [
    {
      streamIndex: 1,
      codec: 'aac',
      channels: 2,
      sampleRate: 48000,
      language: 'en',
      title: 'Main audio',
      isDefault: true,
    },
  ],
  streams: [
    { index: 0, codecType: 'video', codecName: 'h264', durationMs: 184000 },
    { index: 1, codecType: 'audio', codecName: 'aac', language: 'en', durationMs: 184000 },
  ],
};

const storyProjects: Project[] = [
  {
    id: 'youtube-ready',
    title: 'YouTube project',
    status: 'processing',
    source: { kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=storybook' },
    metadata: null,
    createdAt,
    updatedAt,
  },
  {
    id: 'local-unavailable',
    title: 'Local interview',
    status: 'source_imported',
    source: {
      kind: 'managedLocalFile',
      artifactId: 'artifact-local-interview',
      originalFilename: 'local-interview.mp4',
    },
    metadata: storyMediaMetadata,
    createdAt,
    updatedAt,
  },
];

const storyJobs: Job[] = [
  {
    id: 'job-youtube-ready',
    revision: 2,
    projectId: 'youtube-ready',
    title: 'Subtitle import',
    status: 'running',
    stage: 'extractOrGenerateTranscript',
    progress: {
      percent: 42,
      message: 'Importing subtitles',
      currentStep: 'subtitle-import',
      processedItems: null,
      totalItems: null,
    },
    error: null,
    createdAt,
    updatedAt,
  },
];

export function installTauriStoryAdapter() {
  mockIPC(async (cmd, payload) => {
    if (cmd === 'plugin:event|listen' || cmd === 'plugin:event|unlisten') return null;

    switch (cmd) {
      case 'health_check':
        return handleHealthCheck();
      case 'get_transcript_cmd':
        return handleGetTranscript(readPayload<CommandMap['get_transcript_cmd']['args']>(payload));
      case 'list_projects_cmd':
        return handleListProjects();
      case 'get_project_cmd':
        return handleGetProject(readPayload<CommandMap['get_project_cmd']['args']>(payload));
      case 'create_project_cmd':
        return handleCreateProject(readPayload<CommandMap['create_project_cmd']['args']>(payload));
      case 'create_project_from_youtube_cmd':
        return handleCreateProjectFromYoutube(
          readPayload<CommandMap['create_project_from_youtube_cmd']['args']>(payload),
        );
      case 'start_project_mock_pipeline_cmd':
        return handleStartProjectMockPipeline(
          readPayload<CommandMap['start_project_mock_pipeline_cmd']['args']>(payload),
        );
      case 'list_jobs_cmd':
        return handleListJobs();
      case 'list_jobs_snapshot_cmd':
        return handleListJobsSnapshot(
          readPayload<CommandMap['list_jobs_snapshot_cmd']['args']>(payload),
        );
      case 'cancel_job_cmd':
        return handleCancelJob(readPayload<CommandMap['cancel_job_cmd']['args']>(payload));
      case 'probe_local_media_cmd':
        return handleProbeLocalMedia();
      case 'import_local_media_cmd':
        return handleImportLocalMedia(
          readPayload<CommandMap['import_local_media_cmd']['args']>(payload),
        );
      case 'delete_project_cmd':
        return null satisfies CommandMap['delete_project_cmd']['result'];
      default:
        return null;
    }
  });
}

function readPayload<TArgs>(payload: unknown): TArgs {
  return payload as TArgs;
}

function handleHealthCheck(): CommandMap['health_check']['result'] {
  return 'ok';
}

function handleGetTranscript({
  projectId,
}: CommandMap['get_transcript_cmd']['args']): CommandMap['get_transcript_cmd']['result'] {
  return storyTranscripts[projectId] ?? null;
}

function handleListProjects(): CommandMap['list_projects_cmd']['result'] {
  return storyProjects;
}

function handleGetProject({
  projectId,
}: CommandMap['get_project_cmd']['args']): CommandMap['get_project_cmd']['result'] {
  return storyProjects.find((project) => project.id === projectId) ?? storyProjects[0];
}

function handleCreateProject({
  title,
}: CommandMap['create_project_cmd']['args']): CommandMap['create_project_cmd']['result'] {
  return {
    id: 'draft-story-project',
    title,
    status: 'draft',
    source: null,
    metadata: null,
    createdAt,
    updatedAt,
  };
}

function handleCreateProjectFromYoutube({
  url,
}: CommandMap['create_project_from_youtube_cmd']['args']): CommandMap['create_project_from_youtube_cmd']['result'] {
  return createProjectResponse({
    id: 'youtube-created-story',
    title: 'YouTube project',
    status: 'processing',
    source: { kind: 'youtubeUrl', url },
    metadata: null,
    createdAt,
    updatedAt,
  });
}

function handleStartProjectMockPipeline({
  projectId,
}: CommandMap['start_project_mock_pipeline_cmd']['args']): CommandMap['start_project_mock_pipeline_cmd']['result'] {
  const project = handleGetProject({ projectId });
  return createProjectResponse({ ...project, status: 'processing' });
}

function handleListJobs(): CommandMap['list_jobs_cmd']['result'] {
  return storyJobs;
}

function handleListJobsSnapshot({
  projectId,
}: CommandMap['list_jobs_snapshot_cmd']['args']): CommandMap['list_jobs_snapshot_cmd']['result'] {
  return storyJobs.filter((job) => job.projectId === projectId);
}

function handleCancelJob({
  jobId,
}: CommandMap['cancel_job_cmd']['args']): CommandMap['cancel_job_cmd']['result'] {
  const job = storyJobs.find((item) => item.id === jobId) ?? storyJobs[0];
  return {
    ...job,
    status: 'cancelled',
    stage: null,
    progress: {
      percent: job.progress.percent,
      message: 'Cancelled',
      currentStep: null,
      processedItems: null,
      totalItems: null,
    },
  };
}

function handleProbeLocalMedia(): CommandMap['probe_local_media_cmd']['result'] {
  return storyMediaMetadata;
}

function handleImportLocalMedia({
  projectId,
}: CommandMap['import_local_media_cmd']['args']): CommandMap['import_local_media_cmd']['result'] {
  return {
    id: projectId,
    title: 'local-interview.mp4',
    status: 'source_imported',
    source: {
      kind: 'managedLocalFile',
      artifactId: 'artifact-local-interview',
      originalFilename: 'local-interview.mp4',
    },
    metadata: storyMediaMetadata,
    createdAt,
    updatedAt,
  };
}

function createProjectResponse(project: Project): CreateProjectResponse {
  return {
    project,
    job: {
      ...storyJobs[0],
      id: `job-${project.id}`,
      projectId: project.id,
    },
  };
}
