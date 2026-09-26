// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TranslationPublicationNotice } from './TranslationPublicationNotice';
import { context, deferred, id, publication, view } from './tests/support/historicalEdit';
import type { TranslationPublicationView } from '@/shared/api/contracts/translationHistoricalEdit';

const getTranslationPublication = vi.hoisted(() => vi.fn());
vi.mock('@/entities/translation', () => ({ getTranslationPublication }));
const props = {
  projectId: context.projectId,
  translationId: context.translationId,
  resultId: publication.resultId,
  refreshRevision: 0,
};
const ready = {
  ...view,
  publication: { ...publication, state: 'ready' as const },
  link: {
    ...view.link,
    selectedResultId: publication.resultId,
    outputArtifactId: publication.artifactId,
  },
};
beforeEach(() => {
  getTranslationPublication.mockReset();
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe('historical publication notice', () => {
  it('polls pending publication and stops after readiness or unmount', async () => {
    vi.useFakeTimers();
    getTranslationPublication.mockResolvedValueOnce(view).mockResolvedValue(ready);
    const rendered = render(<TranslationPublicationNotice {...props} />);
    await act(async () => {});
    expect(screen.getByRole('status').textContent).toContain('Ожидаем готовности');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000);
    });
    expect(screen.getByRole('status').textContent).toContain('выбрана для проекта');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(6000);
    });
    expect(getTranslationPublication).toHaveBeenCalledTimes(2);
    rendered.unmount();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(6000);
    });
    expect(getTranslationPublication).toHaveBeenCalledTimes(2);
  });
  it('refreshes a ready version after a later project choice and keeps failures visible', async () => {
    getTranslationPublication.mockResolvedValue(ready);
    const rendered = render(<TranslationPublicationNotice {...props} />);
    await screen.findByText(/выбрана для проекта/);
    getTranslationPublication.mockResolvedValue({ ...ready, link: view.link });
    rendered.rerender(<TranslationPublicationNotice {...props} refreshRevision={1} />);
    await screen.findByText(/готова в истории/);
    getTranslationPublication.mockRejectedValue(new Error('Storage unavailable'));
    fireEvent.click(screen.getByRole('button', { name: 'Проверить готовность версии' }));
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Storage unavailable');
    expect(screen.queryByText(/готова в истории/)).toBeNull();
  });
  it('discards a late response and the prior ready status after changing result', async () => {
    getTranslationPublication.mockResolvedValueOnce(ready);
    const next = deferred<TranslationPublicationView>();
    getTranslationPublication.mockReturnValueOnce(next.promise);
    const rendered = render(<TranslationPublicationNotice {...props} />);
    await screen.findByText(/выбрана для проекта/);
    rendered.rerender(<TranslationPublicationNotice {...props} resultId={id(11)} />);
    expect(screen.queryByText(/выбрана для проекта/)).toBeNull();
    getTranslationPublication.mockResolvedValue({
      ...view,
      publication: { ...publication, resultId: id(12), state: 'ready' },
    });
    rendered.rerender(<TranslationPublicationNotice {...props} resultId={id(12)} />);
    await screen.findByText(/готова в истории/);
    await act(async () => {
      next.resolve(ready);
    });
    expect(screen.getByRole('status').textContent).toContain('готова в истории');
    await waitFor(() => expect(getTranslationPublication).toHaveBeenCalledTimes(3));
  });
});
