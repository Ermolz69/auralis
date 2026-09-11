// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { SettingsPage } from '@/pages/settings';
import { NativeThemeProvider as ThemeProvider } from './NativeThemeProvider';
import { AppUpdateProvider, type AppUpdateClient } from '@/features/app-update';

afterEach(() => cleanup());
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke }));
beforeEach(() => {
  localStorage.clear();
  invoke
    .mockReset()
    .mockImplementation(async (command: string, args?: { value: string }) =>
      command === 'get_color_theme_cmd' ? null : { value: args!.value, revision: 1 },
    );
});

describe('SettingsPage', () => {
  it('switches and persists the application color theme', async () => {
    renderSettings();

    const themeSelect = screen.getByLabelText('Color theme');
    fireEvent.change(themeSelect, { target: { value: 'frost' } });

    expect(document.documentElement.getAttribute('data-color-theme')).toBe('frost');
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('set_color_theme_cmd', {
        value: 'frost',
        expectedRevision: 0,
      }),
    );
    expect(localStorage.getItem('auralis:color-theme:v1')).toBeNull();
  });

  it('keeps unsupported settings visibly unavailable', () => {
    renderSettings();

    expect(screen.getByLabelText('Export defaults unavailable')).not.toBeNull();
    expect(screen.getAllByText('Unavailable')).toHaveLength(1);
    expect(screen.getAllByText(/not part of the current app contract/i)).toHaveLength(1);
  });
});

function renderSettings() {
  const client: AppUpdateClient = {
    isSupported: () => false,
    getCurrentVersion: async () => '0.1.0',
    check: async () => null,
    relaunch: async () => undefined,
  };
  return render(
    <ThemeProvider>
      <AppUpdateProvider client={client}>
        <SettingsPage />
      </AppUpdateProvider>
    </ThemeProvider>,
  );
}
