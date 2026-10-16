import { describe, expect, it } from 'vitest';
import { IpcContractError, parseCommandResult } from './runtimeValidation';

const projectId = '00000000-0000-4000-8000-000000000001';
const translationId = '00000000-0000-4000-8000-000000000002';
const runId = '00000000-0000-4000-8000-000000000003';
const link = {
  translationId,
  projectId,
  sourceArtifactId: '00000000-0000-4000-8000-000000000004',
  sourceSha256: 'a'.repeat(64),
  sourceFormat: 'srt',
  sourceLanguage: 'zh',
  targetLanguage: 'ru',
  activeRunId: runId,
  selectedResultId: null,
  outputArtifactId: null,
  revision: 1,
  createdAt: '2026-10-16T00:00:00Z',
  updatedAt: '2026-10-16T00:00:00Z',
};
const run = {
  runId,
  phase: 'running',
  pauseRequested: false,
  savedBlocks: 1,
  totalBlocks: 2,
  resultId: null,
  needsReview: null,
  activeHostJobId: null,
};
const snapshot = { projectId, entries: [{ link, run }] };

describe('project translation snapshot IPC validation', () => {
  it('accepts committed active and inactive links', () => {
    expect(parseCommandResult('get_project_translation_snapshot_cmd', snapshot)).toEqual(snapshot);
    expect(
      parseCommandResult('get_project_translation_snapshot_cmd', {
        projectId,
        entries: [{ link: { ...link, activeRunId: null }, run: null }],
      }),
    ).toHaveProperty('entries');
  });

  it('rejects cross-project links, duplicate translations and mismatched runs', () => {
    for (const invalid of [
      { ...snapshot, entries: [{ link: { ...link, projectId: runId }, run }] },
      { ...snapshot, entries: [snapshot.entries[0], snapshot.entries[0]] },
      { ...snapshot, entries: [{ link, run: { ...run, runId: translationId } }] },
      { ...snapshot, entries: [{ link, run: null }] },
      { ...snapshot, entries: [{ link: { ...link, activeRunId: null }, run }] },
      { ...snapshot, entries: [{ link, run: { ...run, savedBlocks: 3 } }] },
    ]) {
      expect(() => parseCommandResult('get_project_translation_snapshot_cmd', invalid)).toThrow(
        IpcContractError,
      );
    }
  });
});
