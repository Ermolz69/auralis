import { describe, expect, it } from 'vitest';
import { parseCommandResult, IpcContractError } from './runtimeValidation';

const id = (n: number) => `00000000-0000-4000-8000-${n.toString().padStart(12, '0')}`;
const link = {
  projectId: id(1),
  translationId: id(2),
  sourceArtifactId: id(3),
  sourceSha256: 'a'.repeat(64),
  sourceFormat: 'srt',
  sourceLanguage: 'zh',
  targetLanguage: 'ru',
  activeRunId: id(4),
  selectedResultId: id(5),
  outputArtifactId: id(6),
  revision: 2,
  createdAt: '2026-09-26T00:00:00Z',
  updatedAt: '2026-09-26T00:00:00Z',
};
const entry = {
  resultId: id(5),
  runId: id(4),
  artifactId: id(6),
  resultRevision: 1,
  needsReview: true,
};
const page = { link, entries: [entry], nextCursor: null };

describe('translation history IPC contracts', () => {
  it('accepts bounded history, empty history, matching cursors and selected links', () => {
    expect(parseCommandResult('list_translation_history_cmd', page)).toEqual(page);
    expect(
      parseCommandResult('list_translation_history_cmd', { ...page, entries: [] }),
    ).toMatchObject({ entries: [] });
    expect(
      parseCommandResult('list_translation_history_cmd', { ...page, nextCursor: entry.resultId }),
    ).toMatchObject({ nextCursor: entry.resultId });
    expect(parseCommandResult('select_translation_result_cmd', link)).toEqual(link);
  });
  it('rejects duplicate IDs, invalid revisions, unbounded pages and unmatched cursors', () => {
    for (const invalid of [
      { ...page, entries: [entry, entry] },
      { ...page, entries: [{ ...entry, resultRevision: 0 }] },
      { ...page, entries: [{ ...entry, resultRevision: Number.MAX_SAFE_INTEGER + 1 }] },
      { ...page, entries: [{ ...entry, needsReview: 'yes' }] },
      {
        ...page,
        entries: Array.from({ length: 101 }, (_, n) => ({ ...entry, resultId: id(n + 20) })),
      },
      { ...page, nextCursor: id(9) },
      { ...page, entries: [], nextCursor: entry.resultId },
      { ...page, link: { ...link, revision: 0 } },
    ])
      expect(() => parseCommandResult('list_translation_history_cmd', invalid)).toThrow(
        IpcContractError,
      );
  });
});
