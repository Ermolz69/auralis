import { beforeEach, describe, expect, it, vi } from 'vitest';
import { listTranslationHistory, selectTranslationResult } from './translationHistory';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ invoke }));
const projectId = 'project';
const translationId = 'translation';
const resultId = 'result';
const link = { projectId, translationId, selectedResultId: resultId, revision: 8 };
beforeEach(() => invoke.mockReset());

describe('translation history API scope', () => {
  it('sends explicit bounded cursor arguments', async () => {
    invoke.mockResolvedValue({ link, entries: [], nextCursor: null });
    await listTranslationHistory(projectId, translationId, 'cursor', 10);
    expect(invoke).toHaveBeenCalledWith('list_translation_history_cmd', {
      projectId,
      translationId,
      beforeResultId: 'cursor',
      limit: 10,
    });
  });
  it('rejects a validly shaped history response from another project or translation', async () => {
    for (const changed of [{ projectId: 'other' }, { translationId: 'other' }]) {
      invoke.mockResolvedValue({ link: { ...link, ...changed }, entries: [], nextCursor: null });
      await expect(listTranslationHistory(projectId, translationId)).rejects.toThrow(
        'another project',
      );
    }
  });
  it('checks selection identity and sends the observed link revision', async () => {
    invoke.mockResolvedValue(link);
    expect(await selectTranslationResult(projectId, translationId, resultId, 7)).toEqual(link);
    expect(invoke).toHaveBeenCalledWith('select_translation_result_cmd', {
      projectId,
      translationId,
      resultId,
      expectedLinkRevision: 7,
    });
    for (const changed of [
      { projectId: 'other' },
      { translationId: 'other' },
      { selectedResultId: 'other' },
    ]) {
      invoke.mockResolvedValue({ ...link, ...changed });
      await expect(selectTranslationResult(projectId, translationId, resultId, 7)).rejects.toThrow(
        'differs from the requested',
      );
    }
  });
});
