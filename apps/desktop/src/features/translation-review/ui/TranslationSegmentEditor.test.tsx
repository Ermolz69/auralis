// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { TranslationSegmentEditor } from './TranslationSegmentEditor';
import { context, deferred, id, publication, segment } from './tests/support/historicalEdit';
import type { HistoricalTranslationEditContext } from '@/shared/api/contracts/translationHistoricalEdit';

const editHistoricalTranslationResult = vi.hoisted(() => vi.fn());
const getHistoricalTranslationEditContext = vi.hoisted(() => vi.fn());
const publishTranslationResult = vi.hoisted(() => vi.fn());
vi.mock('@/entities/translation', () => ({
  editHistoricalTranslationResult,
  getHistoricalTranslationEditContext,
  publishTranslationResult,
}));
const props = {
  projectId: context.projectId,
  translationId: context.translationId,
  baseResultId: context.baseResultId,
  segment,
};

async function open() {
  fireEvent.click(screen.getByRole('button', { name: 'Исправить перевод' }));
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Сохранить новую версию' }).hasAttribute('disabled'),
    ).toBe(false),
  );
  fireEvent.change(screen.getByRole('textbox', { name: 'Строка перевода 1' }), {
    target: { value: 'Здравствуйте.' },
  });
}
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
beforeEach(() => {
  vi.resetAllMocks();
  vi.stubGlobal('crypto', {
    randomUUID: vi.fn().mockReturnValueOnce(id(10)).mockReturnValue(id(11)),
  });
  getHistoricalTranslationEditContext.mockResolvedValue(context);
  editHistoricalTranslationResult.mockImplementation(async (input) => input.resultId);
  publishTranslationResult.mockResolvedValue(publication);
});

describe('historical TranslationSegmentEditor', () => {
  it('retries publication with the same frozen guards, lines and UUID without another edit', async () => {
    publishTranslationResult
      .mockRejectedValueOnce(new Error('Publication unavailable'))
      .mockResolvedValueOnce(publication);
    const onPublished = vi.fn();
    render(<TranslationSegmentEditor {...props} onPublished={onPublished} />);
    await open();
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить новую версию' }));
    expect(await screen.findByRole('alert')).toHaveProperty(
      'textContent',
      'Publication unavailable',
    );
    expect(editHistoricalTranslationResult).toHaveBeenCalledWith({
      projectId: context.projectId,
      translationId: context.translationId,
      baseResultId: context.baseResultId,
      resultId: id(10),
      expectedHeadResultId: context.expectedHeadResultId,
      expectedLinkRevision: context.expectedLinkRevision,
      segmentId: 1,
      lines: ['Здравствуйте.'],
    });
    fireEvent.click(screen.getByRole('button', { name: 'Повторить сохранение' }));
    await waitFor(() => expect(onPublished).toHaveBeenCalledWith(publication));
    expect(editHistoricalTranslationResult).toHaveBeenCalledTimes(1);
    expect(publishTranslationResult).toHaveBeenCalledTimes(2);
    expect(publishTranslationResult).toHaveBeenLastCalledWith(
      context.projectId,
      context.translationId,
      id(10),
    );
    expect(getHistoricalTranslationEditContext).toHaveBeenCalledTimes(1);
    expect(
      screen.getByRole('textbox', { name: 'Строка перевода 1' }).hasAttribute('disabled'),
    ).toBe(true);
  });
  it('prevents duplicate saves and finishes a committed publication without delivering a stale UI callback', async () => {
    const edit = deferred<string>();
    editHistoricalTranslationResult.mockReturnValue(edit.promise);
    const onPublished = vi.fn();
    const rendered = render(<TranslationSegmentEditor {...props} onPublished={onPublished} />);
    await open();
    const save = screen.getByRole('button', { name: 'Сохранить новую версию' });
    fireEvent.click(save);
    fireEvent.click(save);
    expect(editHistoricalTranslationResult).toHaveBeenCalledTimes(1);
    rendered.rerender(
      <TranslationSegmentEditor {...props} baseResultId={id(12)} onPublished={onPublished} />,
    );
    await act(async () => {
      edit.resolve(id(10));
    });
    await waitFor(() =>
      expect(publishTranslationResult).toHaveBeenCalledWith(
        context.projectId,
        context.translationId,
        id(10),
      ),
    );
    expect(onPublished).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Исправить перевод' })).not.toBeNull();
  });
  it('requires a new observation and UUID after a confirmed precommit head conflict', async () => {
    editHistoricalTranslationResult.mockRejectedValueOnce({
      code: 'CONFLICT',
      message: 'Head changed',
    });
    getHistoricalTranslationEditContext.mockResolvedValueOnce(context).mockResolvedValueOnce({
      ...context,
      expectedHeadResultId: id(15),
      headRevision: 3,
      expectedLinkRevision: 4,
    });
    publishTranslationResult.mockResolvedValue({ ...publication, resultId: id(11) });
    const onPublished = vi.fn();
    render(<TranslationSegmentEditor {...props} onPublished={onPublished} />);
    await open();
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить новую версию' }));
    await screen.findByText('Head changed');
    expect(publishTranslationResult).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Обновить версию для правки' }));
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Сохранить новую версию' }).hasAttribute('disabled'),
      ).toBe(false),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить новую версию' }));
    await waitFor(() => expect(onPublished).toHaveBeenCalledTimes(1));
    expect(editHistoricalTranslationResult.mock.calls[1][0]).toMatchObject({
      resultId: id(11),
      expectedHeadResultId: id(15),
      expectedLinkRevision: 4,
      lines: ['Здравствуйте.'],
    });
  });
  it('keeps the exact request retryable when a save response is lost', async () => {
    editHistoricalTranslationResult.mockRejectedValueOnce(new Error('Lost response'));
    render(<TranslationSegmentEditor {...props} onPublished={vi.fn()} />);
    await open();
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить новую версию' }));
    await screen.findByText('Lost response');
    expect(screen.queryByRole('button', { name: 'Обновить версию для правки' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Повторить сохранение' }));
    await screen.findByText(/Правка сохранена отдельной версией/);
    expect(editHistoricalTranslationResult.mock.calls[1][0]).toEqual(
      editHistoricalTranslationResult.mock.calls[0][0],
    );
  });
  it('keeps save unavailable after a failed context or obsolete late context response', async () => {
    const late = deferred<HistoricalTranslationEditContext>();
    getHistoricalTranslationEditContext
      .mockReturnValueOnce(late.promise)
      .mockRejectedValueOnce(new Error('Base unavailable'));
    const rendered = render(<TranslationSegmentEditor {...props} onPublished={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Исправить перевод' }));
    expect(
      screen.getByRole('button', { name: 'Сохранить новую версию' }).hasAttribute('disabled'),
    ).toBe(true);
    rendered.rerender(
      <TranslationSegmentEditor {...props} baseResultId={id(12)} onPublished={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Исправить перевод' }));
    await screen.findByText('Base unavailable');
    await act(async () => {
      late.resolve(context);
    });
    expect(
      screen.getByRole('button', { name: 'Сохранить новую версию' }).hasAttribute('disabled'),
    ).toBe(true);
    expect(editHistoricalTranslationResult).not.toHaveBeenCalled();
  });
  it('refuses a mismatched result identity before publication', async () => {
    editHistoricalTranslationResult.mockResolvedValue(id(99));
    render(<TranslationSegmentEditor {...props} onPublished={vi.fn()} />);
    await open();
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить новую версию' }));
    await screen.findByText('Сохранённая версия не совпадает с запросом');
    expect(publishTranslationResult).not.toHaveBeenCalled();
  });
});
