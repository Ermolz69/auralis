// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { TranslationSegmentEditor } from './TranslationSegmentEditor';

const editTranslationResult = vi.hoisted(() => vi.fn());
const publishTranslationResult = vi.hoisted(() => vi.fn());
vi.mock('@/entities/translation', () => ({ editTranslationResult, publishTranslationResult }));

const segment = {
  id: 1,
  ordinal: 0,
  cueLabel: null,
  startMs: 1000,
  endMs: 2000,
  sourceLines: ['你好。'],
  translatedLines: ['Привет.'],
};

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
beforeEach(() => {
  vi.clearAllMocks();
  vi.stubGlobal('crypto', { randomUUID: () => '00000000-0000-4000-8000-000000000010' });
});

describe('TranslationSegmentEditor', () => {
  it('retries publication with the same immutable edit ID after a transient failure', async () => {
    editTranslationResult.mockResolvedValue('00000000-0000-4000-8000-000000000010');
    publishTranslationResult
      .mockRejectedValueOnce(new Error('Publication unavailable'))
      .mockResolvedValueOnce({ state: 'pending' });
    const onPublished = vi.fn();
    render(
      <TranslationSegmentEditor
        projectId="project-1"
        translationId="translation-1"
        baseResultId="result-1"
        segment={segment}
        onPublished={onPublished}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Исправить перевод' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Строка перевода 1' }), {
      target: { value: 'Здравствуйте.' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить новую версию' }));
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Publication unavailable');
    expect(editTranslationResult).toHaveBeenCalledWith({
      projectId: 'project-1',
      translationId: 'translation-1',
      baseResultId: 'result-1',
      resultId: '00000000-0000-4000-8000-000000000010',
      segmentId: 1,
      lines: ['Здравствуйте.'],
    });
    fireEvent.click(screen.getByRole('button', { name: 'Повторить сохранение' }));
    await waitFor(() => expect(onPublished).toHaveBeenCalledTimes(1));
    expect(editTranslationResult).toHaveBeenCalledTimes(1);
    expect(publishTranslationResult).toHaveBeenCalledTimes(2);
    expect(publishTranslationResult).toHaveBeenLastCalledWith(
      'project-1', 'translation-1', '00000000-0000-4000-8000-000000000010',
    );
    expect(screen.getByRole('textbox', { name: 'Строка перевода 1' }).hasAttribute('disabled')).toBe(true);
  });
});
