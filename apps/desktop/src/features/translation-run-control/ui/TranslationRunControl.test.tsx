// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { TranslationRunControl } from './TranslationRunControl';

const listProjectTranslations = vi.hoisted(() => vi.fn());
const getTranslationRunStatus = vi.hoisted(() => vi.fn());
const getProjectTranslationSnapshot = vi.hoisted(() => vi.fn());
const startTranslationRun = vi.hoisted(() => vi.fn());
const pauseTranslationRun = vi.hoisted(() => vi.fn());
const publishTranslationResult = vi.hoisted(() => vi.fn());
const subscribeTranslationChanges = vi.hoisted(() => vi.fn());
const listTranslationHistory = vi.hoisted(() => vi.fn());
let notifyChange: () => void;
vi.mock('@/entities/translation', () => ({
  listProjectTranslations,
  getTranslationRunStatus,
  getProjectTranslationSnapshot,
  startTranslationRun,
  pauseTranslationRun,
  publishTranslationResult,
  subscribeTranslationChanges,
  listTranslationHistory,
}));

const projectId = '00000000-0000-4000-8000-000000000001';
const translationId = '00000000-0000-4000-8000-000000000002';
const link = {
  translationId,
  projectId,
  activeRunId: '00000000-0000-4000-8000-000000000003',
  selectedResultId: null,
};
const requested = {
  runId: link.activeRunId,
  phase: 'requested',
  pauseRequested: false,
  savedBlocks: 0,
  totalBlocks: 2,
  resultId: null,
  needsReview: null,
  activeHostJobId: null,
};

afterEach(() => cleanup());
beforeEach(() => {
  vi.clearAllMocks();
  subscribeTranslationChanges.mockImplementation(async (_projectId, notify) => {
    notifyChange = notify;
    return () => {};
  });
  listProjectTranslations.mockResolvedValue([link]);
  listTranslationHistory.mockResolvedValue({ entries: [], nextCursor: null });
  getTranslationRunStatus.mockResolvedValue(requested);
  getProjectTranslationSnapshot.mockImplementation(async (requestedProjectId: string) => {
    const links = await listProjectTranslations(requestedProjectId);
    return {
      projectId: requestedProjectId,
      entries: await Promise.all(
        links.map(async (item: typeof link) => ({
          link: item,
          run: item.activeRunId
            ? await getTranslationRunStatus(requestedProjectId, item.translationId)
            : null,
        })),
      ),
    };
  });
  startTranslationRun.mockResolvedValue({
    state: 'accepted',
    runId: link.activeRunId,
    jobId: '00000000-0000-4000-8000-000000000006',
  });
  pauseTranslationRun.mockResolvedValue(null);
  publishTranslationResult.mockResolvedValue({ state: 'pending' });
});

describe('TranslationRunControl', () => {
  it('refreshes ready history when a detached result appears without changing the project selection', async () => {
    const selected = {
      ...link,
      activeRunId: null,
      selectedResultId: '00000000-0000-4000-8000-000000000004',
      outputArtifactId: '00000000-0000-4000-8000-000000000005',
    };
    listProjectTranslations.mockResolvedValue([selected]);
    listTranslationHistory.mockResolvedValue({
      entries: [{ resultId: selected.selectedResultId }],
      nextCursor: null,
    });
    const changed = vi.fn();
    render(<TranslationRunControl projectId={projectId} onReadyResultsChanged={changed} />);
    await waitFor(() => expect(changed).toHaveBeenCalledTimes(1));
    listTranslationHistory.mockResolvedValue({
      entries: [{ resultId: '00000000-0000-4000-8000-000000000007' }],
      nextCursor: null,
    });
    await act(async () => notifyChange());
    expect(changed).toHaveBeenCalledTimes(2);
    await act(async () => notifyChange());
    expect(changed).toHaveBeenCalledTimes(2);
  });
  it('updates committed progress and ready comparison from events before any polling tick', async () => {
    const onReadyResultsChanged = vi.fn();
    render(
      <TranslationRunControl projectId={projectId} onReadyResultsChanged={onReadyResultsChanged} />,
    );
    await screen.findByRole('button', { name: 'Начать перевод' });
    getTranslationRunStatus.mockResolvedValue({ ...requested, phase: 'running', savedBlocks: 1 });
    await act(async () => notifyChange());
    expect(screen.getByText('Переводится: 1/2')).not.toBeNull();
    listProjectTranslations.mockResolvedValue([
      {
        ...link,
        selectedResultId: '00000000-0000-4000-8000-000000000004',
        outputArtifactId: '00000000-0000-4000-8000-000000000005',
      },
    ]);
    await act(async () => notifyChange());
    expect(onReadyResultsChanged).toHaveBeenCalledTimes(1);
    expect(listProjectTranslations).toHaveBeenCalledTimes(3);
  });
  it('keeps accepted preparation visible while the first refreshed status is delayed', async () => {
    let releaseStatus: (value: unknown) => void = () => {
      throw new Error('No status read');
    };
    const pendingStatus = new Promise((resolve) => {
      releaseStatus = resolve;
    });
    getTranslationRunStatus
      .mockResolvedValueOnce(requested)
      .mockImplementation(() => pendingStatus);
    render(<TranslationRunControl projectId={projectId} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Начать перевод' }));
    await waitFor(() => expect(getTranslationRunStatus).toHaveBeenCalledTimes(2));
    expect(screen.getByText('Подготовка модели: 0/2')).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Пауза' })).not.toBeNull();
    expect(screen.queryByRole('button', { name: 'Начать перевод' })).toBeNull();
    releaseStatus({ ...requested, phase: 'running', savedBlocks: 1 });
    expect(await screen.findByText('Переводится: 1/2')).not.toBeNull();
  });

  it('restores preparation from an accepted host job and allows pause before a Translate attempt', async () => {
    for (const phase of ['requested', 'paused'] as const) {
      getTranslationRunStatus.mockResolvedValue({
        ...requested,
        phase,
        activeHostJobId: '00000000-0000-4000-8000-000000000006',
      });
      const view = render(<TranslationRunControl projectId={projectId} />);
      expect(await screen.findByText('Подготовка модели: 0/2')).not.toBeNull();
      expect(
        screen.queryByRole('button', { name: /Начать перевод|Продолжить перевод/ }),
      ).toBeNull();
      view.unmount();
      render(<TranslationRunControl projectId={projectId} />);
      expect(await screen.findByText('Подготовка модели: 0/2')).not.toBeNull();
      getTranslationRunStatus.mockResolvedValue({ ...requested, phase: 'paused' });
      fireEvent.click(screen.getByRole('button', { name: 'Пауза' }));
      expect(await screen.findByText('На паузе: 0/2')).not.toBeNull();
      expect(
        screen.getByRole('button', { name: 'Продолжить перевод' }).hasAttribute('disabled'),
      ).toBe(false);
      expect(startTranslationRun).not.toHaveBeenCalled();
      cleanup();
    }
  });

  it('keeps preparation visible after accepting start or resume until the worker begins its attempt', async () => {
    for (const phase of ['requested', 'paused'] as const) {
      getTranslationRunStatus.mockResolvedValue({ ...requested, phase });
      startTranslationRun.mockImplementationOnce(async () => {
        getTranslationRunStatus.mockResolvedValue({
          ...requested,
          phase,
          activeHostJobId: '00000000-0000-4000-8000-000000000006',
        });
        return {
          state: 'accepted',
          runId: link.activeRunId,
          jobId: '00000000-0000-4000-8000-000000000006',
        };
      });
      render(<TranslationRunControl projectId={projectId} />);
      fireEvent.click(
        await screen.findByRole('button', {
          name: phase === 'requested' ? 'Начать перевод' : 'Продолжить перевод',
        }),
      );
      await waitFor(() => expect(getTranslationRunStatus).toHaveBeenCalledTimes(2));
      expect(screen.getByText('Подготовка модели: 0/2')).not.toBeNull();
      expect(screen.getByRole('button', { name: 'Пауза' })).not.toBeNull();
      getTranslationRunStatus.mockResolvedValue({ ...requested, phase: 'running', savedBlocks: 1 });
      fireEvent.click(screen.getByRole('button', { name: 'Обновить перевод' }));
      expect(await screen.findByText('Переводится: 1/2')).not.toBeNull();
      cleanup();
      vi.clearAllMocks();
    }
  });

  it('allows pause during initial preparation and paused-run resume without reporting cancellation as an error', async () => {
    for (const phase of ['requested', 'paused'] as const) {
      let rejectStart: (reason: unknown) => void = () => {
        throw new Error('Start was not called');
      };
      startTranslationRun.mockImplementationOnce(
        () =>
          new Promise((_, reject) => {
            rejectStart = reject;
          }),
      );
      getTranslationRunStatus.mockResolvedValue({ ...requested, phase });
      render(<TranslationRunControl projectId={projectId} />);
      fireEvent.click(
        await screen.findByRole('button', {
          name: phase === 'requested' ? 'Начать перевод' : 'Продолжить перевод',
        }),
      );
      expect(await screen.findByText('Подготовка модели: 0/2')).not.toBeNull();
      getTranslationRunStatus.mockResolvedValue({ ...requested, phase: 'paused' });
      fireEvent.click(await screen.findByRole('button', { name: 'Пауза' }));
      await waitFor(() =>
        expect(pauseTranslationRun).toHaveBeenCalledWith(projectId, translationId),
      );
      rejectStart({ code: 'CANCELLED', message: 'The operation was cancelled' });
      await screen.findByText('На паузе: 0/2');
      expect(screen.queryByRole('alert')).toBeNull();
      expect(
        screen.getByRole('button', { name: 'Продолжить перевод' }).hasAttribute('disabled'),
      ).toBe(false);
      cleanup();
    }
  });
  it('starts a prepared run and shows committed progress', async () => {
    render(<TranslationRunControl projectId={projectId} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Начать перевод' }));
    await waitFor(() => expect(startTranslationRun).toHaveBeenCalledWith(projectId, translationId));
    getTranslationRunStatus.mockResolvedValue({ ...requested, phase: 'running', savedBlocks: 1 });
    fireEvent.click(screen.getByRole('button', { name: 'Обновить перевод' }));
    expect(await screen.findByText('Переводится: 1/2')).not.toBeNull();
  });

  it('requests pause and distinguishes it from an acknowledged pause', async () => {
    getTranslationRunStatus.mockResolvedValue({ ...requested, phase: 'running', savedBlocks: 1 });
    render(<TranslationRunControl projectId={projectId} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Пауза' }));
    await waitFor(() => expect(pauseTranslationRun).toHaveBeenCalledWith(projectId, translationId));
    getTranslationRunStatus.mockResolvedValue({
      ...requested,
      phase: 'running',
      pauseRequested: true,
      savedBlocks: 1,
    });
    fireEvent.click(screen.getByRole('button', { name: 'Обновить перевод' }));
    expect(await screen.findByText(/Запрошена пауза/)).not.toBeNull();
    expect(screen.queryByRole('button', { name: 'Пауза' })).toBeNull();
  });

  it('can retry publication of a validated result', async () => {
    const resultId = '00000000-0000-4000-8000-000000000004';
    getTranslationRunStatus.mockResolvedValue({
      ...requested,
      phase: 'validated',
      resultId,
      savedBlocks: 2,
    });
    render(<TranslationRunControl projectId={projectId} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Прикрепить результат' }));
    await waitFor(() =>
      expect(publishTranslationResult).toHaveBeenCalledWith(projectId, translationId, resultId),
    );
  });

  it('announces a newly ready result once so comparison can refresh', async () => {
    const onReadyResultsChanged = vi.fn();
    const resultId = '00000000-0000-4000-8000-000000000004';
    render(
      <TranslationRunControl projectId={projectId} onReadyResultsChanged={onReadyResultsChanged} />,
    );
    await screen.findByRole('button', { name: 'Начать перевод' });
    expect(onReadyResultsChanged).not.toHaveBeenCalled();
    listProjectTranslations.mockResolvedValue([
      {
        ...link,
        selectedResultId: resultId,
        outputArtifactId: '00000000-0000-4000-8000-000000000005',
      },
    ]);
    fireEvent.click(screen.getByRole('button', { name: 'Обновить перевод' }));
    await waitFor(() => expect(onReadyResultsChanged).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByRole('button', { name: 'Обновить перевод' }));
    await waitFor(() => expect(listProjectTranslations).toHaveBeenCalledTimes(3));
    expect(onReadyResultsChanged).toHaveBeenCalledTimes(1);
  });

  it('shows a missing runtime error without losing the prepared run', async () => {
    startTranslationRun.mockRejectedValue({
      code: 'NOT_FOUND',
      message: 'Translation model runtime is not installed',
    });
    render(<TranslationRunControl projectId={projectId} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Начать перевод' }));
    expect(await screen.findByRole('alert')).toHaveProperty(
      'textContent',
      'Translation model runtime is not installed',
    );
    expect(screen.getByRole('button', { name: 'Начать перевод' })).not.toBeNull();
  });
});
