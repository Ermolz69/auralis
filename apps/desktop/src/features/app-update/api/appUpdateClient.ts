import { getVersion } from '@tauri-apps/api/app';
import { isTauri } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type DownloadEvent } from '@tauri-apps/plugin-updater';

export type AppUpdateDownloadEvent = DownloadEvent;

export interface AppUpdateCandidate {
  currentVersion: string;
  version: string;
  date?: string;
  body?: string;
  downloadAndInstall: (onEvent: (event: AppUpdateDownloadEvent) => void) => Promise<void>;
  close: () => Promise<void>;
}

export interface AppUpdateClient {
  isSupported: () => boolean;
  getCurrentVersion: () => Promise<string>;
  check: () => Promise<AppUpdateCandidate | null>;
  relaunch: () => Promise<void>;
}

export const nativeAppUpdateClient: AppUpdateClient = {
  isSupported: isTauri,
  async getCurrentVersion() {
    return isTauri() ? getVersion() : __APP_VERSION__;
  },
  async check() {
    const update = await check({ timeout: 20_000 });
    if (!update) return null;
    return {
      currentVersion: update.currentVersion,
      version: update.version,
      date: update.date,
      body: update.body?.trim().slice(0, 4_000),
      downloadAndInstall: (onEvent) => update.downloadAndInstall(onEvent, { timeout: 15 * 60_000 }),
      close: () => update.close(),
    };
  },
  relaunch,
};
