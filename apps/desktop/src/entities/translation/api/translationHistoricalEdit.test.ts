import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  editHistoricalTranslationResult,
  getHistoricalTranslationEditContext,
  getTranslationPublication,
} from './translationHistoricalEdit';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ invoke }));
const context = {
  projectId: 'project',
  translationId: 'translation',
  baseResultId: 'base',
  runId: 'run',
  baseRevision: 1,
  expectedHeadResultId: 'head',
  headRevision: 2,
  expectedLinkRevision: 7,
};
beforeEach(() => {
  invoke.mockReset();
});

describe('historical translation API identity', () => {
  it('observes an explicit historical base and rejects cross-scope responses', async () => {
    invoke.mockResolvedValue(context);
    expect(await getHistoricalTranslationEditContext('project', 'translation', 'base')).toEqual(
      context,
    );
    expect(invoke).toHaveBeenCalledWith('get_historical_translation_edit_context_cmd', {
      projectId: 'project',
      translationId: 'translation',
      baseResultId: 'base',
    });
    for (const changed of [
      { projectId: 'other' },
      { translationId: 'other' },
      { baseResultId: 'other' },
    ]) {
      invoke.mockResolvedValue({ ...context, ...changed });
      await expect(
        getHistoricalTranslationEditContext('project', 'translation', 'base'),
      ).rejects.toThrow('requested base');
    }
  });
  it('sends the complete branch request and rejects a different committed result', async () => {
    const input = {
      projectId: 'project',
      translationId: 'translation',
      baseResultId: 'base',
      resultId: 'result',
      expectedHeadResultId: 'head',
      expectedLinkRevision: 7,
      segmentId: 1,
      lines: ['Перевод.'],
    };
    invoke.mockResolvedValue('result');
    expect(await editHistoricalTranslationResult(input)).toBe('result');
    expect(invoke).toHaveBeenCalledWith('edit_historical_translation_result_cmd', { input });
    invoke.mockResolvedValue('other');
    await expect(editHistoricalTranslationResult(input)).rejects.toThrow(
      'differs from the request',
    );
  });
  it('reads publication without mutation and rejects another project, translation or result', async () => {
    const view = {
      link: { projectId: 'project', translationId: 'translation' },
      publication: { resultId: 'result' },
    };
    invoke.mockResolvedValue(view);
    expect(await getTranslationPublication('project', 'translation', 'result')).toEqual(view);
    expect(invoke).toHaveBeenCalledWith('get_translation_publication_cmd', {
      projectId: 'project',
      translationId: 'translation',
      resultId: 'result',
    });
    for (const changed of [
      { ...view, link: { ...view.link, projectId: 'other' } },
      { ...view, link: { ...view.link, translationId: 'other' } },
      { ...view, publication: { resultId: 'other' } },
    ]) {
      invoke.mockResolvedValue(changed);
      await expect(getTranslationPublication('project', 'translation', 'result')).rejects.toThrow(
        'requested project result',
      );
    }
  });
});
