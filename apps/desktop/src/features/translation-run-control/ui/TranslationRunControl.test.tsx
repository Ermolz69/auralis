// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { TranslationRunControl } from './TranslationRunControl';

const listProjectTranslations = vi.hoisted(() => vi.fn());
const getTranslationRunStatus = vi.hoisted(() => vi.fn());
const startTranslationRun = vi.hoisted(() => vi.fn());
const pauseTranslationRun = vi.hoisted(() => vi.fn());
const publishTranslationResult = vi.hoisted(() => vi.fn());
vi.mock('@/entities/translation', () => ({
  listProjectTranslations,
  getTranslationRunStatus,
  startTranslationRun,
  pauseTranslationRun,
  publishTranslationResult,
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
};

afterEach(() => cleanup());
beforeEach(() => {
  vi.clearAllMocks();
  listProjectTranslations.mockResolvedValue([link]);
  getTranslationRunStatus.mockResolvedValue(requested);
  startTranslationRun.mockResolvedValue({ state: 'pending' });
  pauseTranslationRun.mockResolvedValue(null);
  publishTranslationResult.mockResolvedValue({ state: 'pending' });
});

describe('TranslationRunControl', () => {
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
    getTranslationRunStatus.mockResolvedValue({ ...requested, phase: 'running', pauseRequested: true, savedBlocks: 1 });
    fireEvent.click(screen.getByRole('button', { name: 'Обновить перевод' }));
    expect(await screen.findByText(/Запрошена пауза/)).not.toBeNull();
    expect(screen.queryByRole('button', { name: 'Пауза' })).toBeNull();
  });

  it('can retry publication of a validated result', async () => {
    const resultId = '00000000-0000-4000-8000-000000000004';
    getTranslationRunStatus.mockResolvedValue({ ...requested, phase: 'validated', resultId, savedBlocks: 2 });
    render(<TranslationRunControl projectId={projectId} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Прикрепить результат' }));
    await waitFor(() => expect(publishTranslationResult).toHaveBeenCalledWith(projectId, translationId, resultId));
  });

  it('shows a missing runtime error without losing the prepared run', async () => {
    startTranslationRun.mockRejectedValue({ code: 'NOT_FOUND', message: 'Translation model runtime is not installed' });
    render(<TranslationRunControl projectId={projectId} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Начать перевод' }));
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Translation model runtime is not installed');
    expect(screen.getByRole('button', { name: 'Начать перевод' })).not.toBeNull();
  });
});
