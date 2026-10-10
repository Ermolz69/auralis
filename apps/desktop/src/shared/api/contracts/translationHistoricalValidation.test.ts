import { describe, expect, it } from 'vitest';
import { IpcContractError, parseCommandResult } from './runtimeValidation';

const id = (n: number) => `00000000-0000-4000-8000-${n.toString().padStart(12, '0')}`;
const context = {
  projectId: id(1),
  translationId: id(2),
  baseResultId: id(3),
  runId: id(4),
  baseRevision: 1,
  expectedHeadResultId: id(5),
  headRevision: 2,
  expectedLinkRevision: 3,
};
const publication = {
  translationId: id(2),
  resultId: id(6),
  runId: id(4),
  artifactId: id(7),
  sourceSha256: 'a'.repeat(64),
  needsReview: true,
  state: 'pending',
};
const link = {
  projectId: id(1),
  translationId: id(2),
  sourceArtifactId: id(8),
  sourceSha256: publication.sourceSha256,
  sourceFormat: 'srt',
  sourceLanguage: 'zh',
  targetLanguage: 'ru',
  activeRunId: id(4),
  selectedResultId: id(5),
  outputArtifactId: id(9),
  revision: 3,
  createdAt: '2026-09-26T00:00:00Z',
  updatedAt: '2026-09-26T00:00:00Z',
};
const view = { link, publication };

describe('historical translation IPC contracts', () => {
  it('accepts a nullable frozen retry request without rewriting its observations or lines', () => {
    const input = {
      projectId: id(1),
      translationId: id(2),
      baseResultId: id(3),
      resultId: id(6),
      expectedHeadResultId: id(5),
      expectedLinkRevision: 3,
      segmentId: 1,
      lines: ['  Exact text  '],
    };
    expect(parseCommandResult('native_e2e_historical_retry_input_cmd', null)).toBeNull();
    expect(parseCommandResult('native_e2e_historical_retry_input_cmd', input)).toEqual(input);
  });

  it('rejects malformed retry fixture IDs, numeric guards and line payloads', () => {
    const input = {
      projectId: id(1),
      translationId: id(2),
      baseResultId: id(3),
      resultId: id(6),
      expectedHeadResultId: id(5),
      expectedLinkRevision: 3,
      segmentId: 1,
      lines: ['Exact text'],
    };
    for (const changed of [
      { resultId: '00000000-0000-0000-0000-000000000000' },
      { expectedHeadResultId: 'invalid' },
      { expectedLinkRevision: Number.MAX_SAFE_INTEGER + 1 },
      { expectedLinkRevision: 0 },
      { segmentId: 0x100000000 },
      { segmentId: 0 },
      { lines: [] },
      { lines: ['valid', 12] },
      { lines: 'text' },
    ])
      expect(() =>
        parseCommandResult('native_e2e_historical_retry_input_cmd', { ...input, ...changed }),
      ).toThrow(IpcContractError);
  });

  it('accepts validated older and current base observations and rejects impossible guards', () => {
    expect(parseCommandResult('get_historical_translation_edit_context_cmd', context)).toEqual(
      context,
    );
    const current = { ...context, baseResultId: context.expectedHeadResultId, baseRevision: 2 };
    expect(parseCommandResult('get_historical_translation_edit_context_cmd', current)).toEqual(
      current,
    );
    for (const changed of [
      { expectedHeadResultId: context.baseResultId },
      { headRevision: 1 },
      { headRevision: 0 },
      { baseRevision: 3 },
      { expectedLinkRevision: 0 },
      { headRevision: Number.MAX_SAFE_INTEGER + 1 },
      { runId: '00000000-0000-0000-0000-000000000000' },
      { projectId: 'invalid' },
    ])
      expect(() =>
        parseCommandResult('get_historical_translation_edit_context_cmd', {
          ...context,
          ...changed,
        }),
      ).toThrow(IpcContractError);
  });
  it('accepts pending, ready detached and ready attached snapshots', () => {
    expect(parseCommandResult('get_translation_publication_cmd', view)).toEqual(view);
    const detached = { ...view, publication: { ...publication, state: 'ready' } };
    expect(parseCommandResult('get_translation_publication_cmd', detached)).toEqual(detached);
    const attached = {
      ...detached,
      link: {
        ...link,
        selectedResultId: publication.resultId,
        outputArtifactId: publication.artifactId,
      },
    };
    expect(parseCommandResult('get_translation_publication_cmd', attached)).toEqual(attached);
  });
  it('rejects a mixed snapshot, pending selection, mismatched artifact and source', () => {
    for (const invalid of [
      { ...view, publication: { ...publication, translationId: id(11) } },
      { ...view, publication: { ...publication, sourceSha256: 'b'.repeat(64) } },
      { ...view, link: { ...link, selectedResultId: publication.resultId } },
      {
        ...view,
        publication: { ...publication, state: 'ready' },
        link: { ...link, selectedResultId: publication.resultId },
      },
    ])
      expect(() => parseCommandResult('get_translation_publication_cmd', invalid)).toThrow(
        IpcContractError,
      );
  });
});
