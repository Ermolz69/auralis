import { describe, expect, it } from 'vitest';
import { IpcContractError, parseCommandResult, parseEventPayload } from './runtimeValidation';

const project = {
  id: '00000000-0000-4000-8000-000000000001',
  title: 'Project',
  status: 'draft',
  source: null,
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

describe('IPC runtime validation', () => {
  it('accepts a valid command payload', () => {
    expect(parseCommandResult('list_projects_cmd', [project])).toEqual([project]);
    expect(
      parseCommandResult('probe_local_media_cmd', {
        durationMs: 1,
        hasVideo: false,
        hasAudio: false,
        streams: [],
        audioTracks: [],
      }),
    ).toMatchObject({ durationMs: 1 });
  });

  it('rejects incompatible command payloads without logging their contents', () => {
    expect(() =>
      parseCommandResult('list_projects_cmd', [{ ...project, status: 'unknown', secret: 'token' }]),
    ).toThrow(new IpcContractError('command', 'list_projects_cmd'));
  });

  it('validates null unit events and structured events', () => {
    expect(parseEventPayload('job-events-invalidated', null)).toBeNull();
    expect(
      parseEventPayload('transcript-ready', {
        projectId: project.id,
        jobId: '00000000-0000-4000-8000-000000000002',
      }),
    ).toEqual({
      projectId: project.id,
      jobId: '00000000-0000-4000-8000-000000000002',
    });
    expect(() => parseEventPayload('job-events-invalidated', undefined)).toThrow(IpcContractError);
  });

  it('rejects unsafe u64 values at the JavaScript boundary', () => {
    expect(() =>
      parseCommandResult('list_project_artifacts_cmd', [
        {
          id: '00000000-0000-4000-8000-000000000003',
          kind: 'downloadedVideo',
          location: { kind: 'storageKey', value: 'project/video.mp4' },
          sizeBytes: Number.MAX_SAFE_INTEGER + 1,
          state: 'ready',
          createdAt: '2026-01-01T00:00:00Z',
          updatedAt: '2026-01-01T00:00:00Z',
          readyAt: null,
        },
      ]),
    ).toThrow(IpcContractError);
  });
});
