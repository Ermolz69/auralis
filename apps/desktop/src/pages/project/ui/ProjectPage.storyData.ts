import type { JobDto } from '@/entities/job';
import type { Project } from '@/entities/project';

export const project: Project = {
  revision: 1,
  id: 'workspace-story',
  title: 'auralis',
  status: 'processing',
  source: { kind: 'youtubeUrl', url: 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' },
  metadata: {
    durationMs: 2892000,
    width: 1920,
    height: 1080,
    fps: 30,
    videoCodec: 'h264',
    container: 'mp4',
    hasVideo: true,
    hasAudio: true,
    audioCodec: 'aac',
    sampleRate: 48000,
    audioChannels: 2,
    audioTracks: [
      {
        streamIndex: 1,
        codec: 'aac',
        channels: 2,
        sampleRate: 48000,
        language: 'en',
        isDefault: true,
      },
    ],
    streams: [
      { index: 0, codecType: 'video', codecName: 'h264', durationMs: 2892000 },
      { index: 1, codecType: 'audio', codecName: 'aac', language: 'en', durationMs: 2892000 },
    ],
  },
  createdAt: '2026-08-02T14:23:05.000Z',
  updatedAt: '2026-08-02T14:24:52.000Z',
};
export const runningJob: JobDto = {
  kind: 'dubbing',
  id: 'job-running',
  revision: 2,
  projectId: project.id,
  title: 'Импорт субтитров',
  status: 'running',
  stage: 'extractOrGenerateTranscript',
  progress: {
    percent: 62,
    message: 'Импорт субтитров',
    currentStep: 'subtitle-import',
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: '2026-08-02T14:24:52.000Z',
  updatedAt: '2026-08-02T14:26:00.000Z',
};
