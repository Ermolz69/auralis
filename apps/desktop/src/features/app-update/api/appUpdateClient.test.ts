import { beforeEach, describe, expect, it, vi } from 'vitest';

const tauri = vi.hoisted(() => ({
  check: vi.fn(),
  getVersion: vi.fn(),
  isTauri: vi.fn(),
  relaunch: vi.fn(),
}));

vi.mock('@tauri-apps/api/app', () => ({ getVersion: tauri.getVersion }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: tauri.isTauri }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: tauri.relaunch }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: tauri.check }));

import { nativeAppUpdateClient } from './appUpdateClient';

beforeEach(() => vi.clearAllMocks());

describe('nativeAppUpdateClient', () => {
  it('uses the bundled version without native IPC in a browser', async () => {
    tauri.isTauri.mockReturnValue(false);

    await expect(nativeAppUpdateClient.getCurrentVersion()).resolves.toBe('0.1.0');
    expect(tauri.getVersion).not.toHaveBeenCalled();
  });

  it('reads the installed version and reports no available release', async () => {
    tauri.isTauri.mockReturnValue(true);
    tauri.getVersion.mockResolvedValue('0.1.0');
    tauri.check.mockResolvedValue(null);

    await expect(nativeAppUpdateClient.getCurrentVersion()).resolves.toBe('0.1.0');
    await expect(nativeAppUpdateClient.check()).resolves.toBeNull();
    expect(tauri.check).toHaveBeenCalledWith({ timeout: 20_000 });
  });

  it('adapts, limits and installs a native updater resource', async () => {
    const update = {
      currentVersion: '0.1.0',
      version: '0.2.0',
      date: '2026-09-05T12:00:00Z',
      body: `  ${'n'.repeat(5_000)}  `,
      downloadAndInstall: vi.fn(async () => undefined),
      close: vi.fn(async () => undefined),
    };
    tauri.check.mockResolvedValue(update);

    const candidate = await nativeAppUpdateClient.check();
    expect(candidate?.body).toHaveLength(4_000);
    const listener = vi.fn();
    await candidate?.downloadAndInstall(listener);
    await candidate?.close();
    await nativeAppUpdateClient.relaunch();

    expect(update.downloadAndInstall).toHaveBeenCalledWith(listener, { timeout: 900_000 });
    expect(update.close).toHaveBeenCalledOnce();
    expect(tauri.relaunch).toHaveBeenCalledOnce();
  });
});
