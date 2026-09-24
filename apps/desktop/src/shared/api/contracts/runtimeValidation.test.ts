import { describe, expect, it } from 'vitest';
import { IpcContractError, parseCommandResult, parseEventPayload } from './runtimeValidation';

const project = {
  id: '00000000-0000-4000-8000-000000000001',
  title: 'Project',
  status: 'draft',
  source: null,
  revision: 1,
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

  it('validates paginated job history cursors', () => {
    const page = {
      jobs: [],
      nextCursor: {
        createdAt: '2026-01-01T00:00:00Z',
        jobId: '00000000-0000-4000-8000-000000000002',
      },
    };
    expect(parseCommandResult('list_job_history_page_cmd', page)).toEqual(page);
    expect(() =>
      parseCommandResult('list_job_history_page_cmd', {
        jobs: [],
        nextCursor: { createdAt: 'not-a-date', jobId: 'job' },
      }),
    ).toThrow(IpcContractError);
  });

  it('accepts bounded translation text slots and rejects ranges outside the source', () => {
    const inspection = {
      format: 'vtt',
      sourceSha256: 'a'.repeat(64),
      sourceSize: 100,
      parserVersion: 1,
      segments: [
        {
          id: 1,
          ordinal: 0,
          cueLabel: null,
          startMs: 1000,
          endMs: 2000,
          sourceLines: ['你好。'],
          textRanges: [[40, 49]],
        },
      ],
    };
    expect(parseCommandResult('inspect_translation_source_cmd', inspection)).toEqual(inspection);
    expect(() =>
      parseCommandResult('inspect_translation_source_cmd', {
        ...inspection,
        segments: [{ ...inspection.segments[0], textRanges: [[40, 101]] }],
      }),
    ).toThrow(IpcContractError);
  });

  it('requires a result identity for a validated translation run', () => {
    const snapshot = {
      runId: '00000000-0000-4000-8000-000000000004',
      phase: 'validated',
      pauseRequested: false,
      savedBlocks: 2,
      totalBlocks: 2,
      resultId: '00000000-0000-4000-8000-000000000005',
      needsReview: true,
    };
    expect(parseCommandResult('get_translation_run_status_cmd', snapshot)).toEqual(snapshot);
    expect(parseCommandResult('pause_translation_run_cmd', null)).toBeNull();
    expect(() =>
      parseCommandResult('get_translation_run_status_cmd', {
        ...snapshot,
        resultId: null,
      }),
    ).toThrow(IpcContractError);
  });

  it('validates project translation links and paired result artifacts', () => {
    const link = {
      translationId: '00000000-0000-4000-8000-000000000011',
      projectId: '00000000-0000-4000-8000-000000000012',
      sourceArtifactId: '00000000-0000-4000-8000-000000000013',
      sourceSha256: 'a'.repeat(64),
      sourceFormat: 'srt',
      sourceLanguage: 'zh',
      targetLanguage: 'ru',
      activeRunId: null,
      selectedResultId: '00000000-0000-4000-8000-000000000014',
      outputArtifactId: '00000000-0000-4000-8000-000000000015',
      revision: 2,
      createdAt: '2026-09-25T00:00:00Z',
      updatedAt: '2026-09-25T00:01:00Z',
    };
    expect(parseCommandResult('list_project_translations_cmd', [link])).toEqual([link]);
    expect(parseCommandResult('begin_translation_run_cmd', link)).toEqual(link);
    expect(() =>
      parseCommandResult('list_project_translations_cmd', [
        { ...link, outputArtifactId: null },
      ]),
    ).toThrow(IpcContractError);
    expect(() =>
      parseCommandResult('list_project_translations_cmd', [
        { ...link, sourceLanguage: 'fr' },
      ]),
    ).toThrow(IpcContractError);
  });

  it('validates translation edit and publication identities', () => {
    const resultId = '00000000-0000-4000-8000-000000000021';
    const publication = {
      translationId: '00000000-0000-4000-8000-000000000022',
      resultId,
      runId: '00000000-0000-4000-8000-000000000023',
      artifactId: '00000000-0000-4000-8000-000000000024',
      sourceSha256: 'b'.repeat(64),
      needsReview: true,
      state: 'pending',
    };
    expect(parseCommandResult('edit_translation_result_cmd', resultId)).toBe(resultId);
    expect(parseCommandResult('publish_translation_result_cmd', publication)).toEqual(publication);
    expect(() => parseCommandResult('edit_translation_result_cmd', 'bad-id')).toThrow(
      IpcContractError,
    );
    expect(() =>
      parseCommandResult('publish_translation_result_cmd', {
        ...publication,
        state: 'unknown',
      }),
    ).toThrow(IpcContractError);
  });

  it('validates bounded source and translated comparison pages', () => {
    const page = {
      resultId: '00000000-0000-4000-8000-000000000031',
      runId: '00000000-0000-4000-8000-000000000032',
      needsReview: true,
      totalSegments: 1,
      offset: 0,
      segments: [{
        id: 1,
        ordinal: 0,
        cueLabel: null,
        startMs: 1000,
        endMs: 2000,
        sourceLines: ['你好。'],
        translatedLines: ['Здравствуйте.'],
      }],
    };
    expect(parseCommandResult('get_translation_comparison_cmd', page)).toEqual(page);
    expect(() => parseCommandResult('get_translation_comparison_cmd', {
      ...page,
      segments: [{ ...page.segments[0], translatedLines: [] }],
    })).toThrow(IpcContractError);
    expect(() => parseCommandResult('get_translation_comparison_cmd', {
      ...page,
      segments: Array(101).fill(page.segments[0]),
    })).toThrow(IpcContractError);
  });

  it('rejects impossible media rates and structurally conflicting transcript segments', () => {
    expect(() =>
      parseCommandResult('probe_local_media_cmd', {
        durationMs: 1,
        fps: -30,
        hasVideo: true,
        hasAudio: false,
        streams: [],
        audioTracks: [],
      }),
    ).toThrow(IpcContractError);

    const segment = {
      id: 'segment-1',
      index: 0,
      startMs: 20,
      endMs: 10,
      sourceText: 'Invalid timing',
    };
    expect(() =>
      parseCommandResult('get_transcript_cmd', {
        language: 'en',
        segments: [segment],
      }),
    ).toThrow(IpcContractError);

    expect(() =>
      parseCommandResult('get_transcript_cmd', {
        language: 'en',
        segments: [
          { ...segment, startMs: 0, endMs: 10 },
          { ...segment, startMs: 10, endMs: 20 },
        ],
      }),
    ).toThrow(IpcContractError);
  });
});
