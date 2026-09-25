// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { TranslationSourceImport } from './TranslationSourceImport';

const open = vi.hoisted(() => vi.fn());
const importTranslationSource = vi.hoisted(() => vi.fn());
const listOriginalSubtitles = vi.hoisted(() => vi.fn());
const listProjectTranslations = vi.hoisted(() => vi.fn());
const beginTranslationRun = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/plugin-dialog', () => ({ open }));
vi.mock('@/entities/translation', () => ({
  beginTranslationRun,
  importTranslationSource,
  listOriginalSubtitles,
  listProjectTranslations,
}));

const projectId = '00000000-0000-4000-8000-000000000001';
const artifact = {
  id: '00000000-0000-4000-8000-000000000002',
  kind: 'originalSubtitle',
  location: { kind: 'storageKey', value: 'source.srt' },
  sizeBytes: 40,
  state: 'pendingFinalize',
  createdAt: '2026-09-25T00:00:00Z',
  updatedAt: '2026-09-25T00:00:00Z',
  readyAt: null,
};

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
beforeEach(() => {
  vi.clearAllMocks();
  listOriginalSubtitles.mockResolvedValueOnce([]).mockResolvedValue([artifact]);
  listProjectTranslations.mockResolvedValue([]);
  importTranslationSource.mockResolvedValue(artifact);
});

describe('TranslationSourceImport', () => {
  it('imports a selected SRT as a managed original and displays pending finalization', async () => {
    open.mockResolvedValue('C:/subtitles/source.SRT');
    render(<TranslationSourceImport projectId={projectId} />);
    await screen.findByText('Исходников пока нет.');
    fireEvent.click(screen.getByRole('button', { name: 'Импортировать SRT/VTT' }));
    await waitFor(() => expect(importTranslationSource).toHaveBeenCalledWith(
      projectId, 'C:/subtitles/source.SRT', 'srt',
    ));
    expect(await screen.findByText('Сохраняется…')).not.toBeNull();
    expect(open).toHaveBeenCalledWith({
      multiple: false,
      filters: [{ name: 'Subtitle file', extensions: ['srt', 'vtt'] }],
    });
  });

  it('does not import a cancelled or unsupported file selection', async () => {
    open.mockResolvedValueOnce(null).mockResolvedValueOnce('C:/subtitles/source.txt');
    render(<TranslationSourceImport projectId={projectId} />);
    await screen.findByText('Исходников пока нет.');
    fireEvent.click(screen.getByRole('button', { name: 'Импортировать SRT/VTT' }));
    await waitFor(() => expect(open).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Импортировать SRT/VTT' }).hasAttribute('disabled')).toBe(false));
    fireEvent.click(screen.getByRole('button', { name: 'Импортировать SRT/VTT' }));
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Выберите файл SRT или VTT');
    expect(importTranslationSource).not.toHaveBeenCalled();
  });

  it('prepares a Chinese run once a managed original is ready', async () => {
    listOriginalSubtitles.mockReset().mockResolvedValue([{ ...artifact, state: 'ready' }]);
    let sequence = 0;
    vi.stubGlobal('crypto', { randomUUID: () => `00000000-0000-4000-8000-${String(++sequence).padStart(12, '0')}` });
    const link = {
      translationId: '00000000-0000-4000-8000-000000000001',
      sourceArtifactId: artifact.id,
      sourceLanguage: 'zh',
    };
    listProjectTranslations.mockReset().mockResolvedValueOnce([]).mockResolvedValue([link]);
    beginTranslationRun.mockResolvedValue(link);
    render(<TranslationSourceImport projectId={projectId} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Подготовить перевод с китайского' }));
    await waitFor(() => expect(beginTranslationRun).toHaveBeenCalledWith({
      projectId,
      sourceArtifactId: artifact.id,
      format: 'srt',
      sourceLanguage: 'zh',
      translationId: '00000000-0000-4000-8000-000000000001',
      runId: '00000000-0000-4000-8000-000000000002',
    }));
    expect(await screen.findByText('Перевод подготовлен')).not.toBeNull();
  });
});
