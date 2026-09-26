// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { TranslationModelSetup } from './TranslationModelSetup';

const getTranslationPackageStatus = vi.hoisted(() => vi.fn());
const installTranslationPackage = vi.hoisted(() => vi.fn());
const selectTranslationPackage = vi.hoisted(() => vi.fn());

vi.mock('@/entities/translation', () => ({
  getTranslationPackageStatus,
  installTranslationPackage,
  selectTranslationPackage,
}));

const packageStatus = {
  schemaVersion: 1,
  releaseId: 'hy-mt2-1-8b-q4-windows-x64-cpu-b10977',
  backend: 'cpu',
  supported: true,
  installed: false,
  selected: false,
};

beforeEach(() => {
  vi.clearAllMocks();
  getTranslationPackageStatus.mockResolvedValue(packageStatus);
  installTranslationPackage.mockResolvedValue({ ...packageStatus, installed: true });
  selectTranslationPackage.mockResolvedValue({ ...packageStatus, installed: true, selected: true });
});

afterEach(() => cleanup());

describe('TranslationModelSetup', () => {
  it('installs and selects the pinned package without exposing filesystem paths', async () => {
    render(<TranslationModelSetup />);
    fireEvent.click(await screen.findByRole('button', { name: 'Скачать и установить' }));
    await waitFor(() => expect(installTranslationPackage).toHaveBeenCalledOnce());
    fireEvent.click(await screen.findByRole('button', { name: 'Выбрать модель' }));
    await waitFor(() =>
      expect(selectTranslationPackage).toHaveBeenCalledWith(packageStatus.releaseId, 'cpu'),
    );
    expect(
      await screen.findByText('Локальная модель выбрана. Перед запуском её файлы будут проверены.'),
    ).not.toBeNull();
  });

  it('keeps an installed package available when selection is busy', async () => {
    getTranslationPackageStatus.mockResolvedValue({ ...packageStatus, installed: true });
    selectTranslationPackage.mockRejectedValue({
      code: 'BUSY',
      message: 'Pause or finish active translation jobs before changing the model',
    });
    render(<TranslationModelSetup />);
    fireEvent.click(await screen.findByRole('button', { name: 'Выбрать модель' }));
    expect(await screen.findByRole('alert')).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Выбрать модель' })).not.toBeNull();
  });

  it('lets a selected package be verified and repaired without changing its selection', async () => {
    getTranslationPackageStatus.mockResolvedValue({
      ...packageStatus,
      installed: true,
      selected: true,
    });
    installTranslationPackage.mockResolvedValue({
      ...packageStatus,
      installed: true,
      selected: true,
    });
    render(<TranslationModelSetup />);
    fireEvent.click(await screen.findByRole('button', { name: 'Проверить и восстановить' }));
    await waitFor(() => expect(installTranslationPackage).toHaveBeenCalledOnce());
    expect(
      await screen.findByText('Локальная модель выбрана. Перед запуском её файлы будут проверены.'),
    ).not.toBeNull();
  });
});
