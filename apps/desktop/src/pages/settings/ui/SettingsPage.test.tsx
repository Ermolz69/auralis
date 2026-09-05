// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { SettingsPage } from './SettingsPage';
import { ThemeProvider } from '../../../shared/theme';
import { AppUpdateProvider, type AppUpdateClient } from '../../../features/app-update';

afterEach(() => cleanup());
beforeEach(() => localStorage.clear());

describe('SettingsPage', () => {
  it('switches and persists the application color theme', () => {
    renderSettings();

    const themeSelect = screen.getByLabelText('Color theme');
    fireEvent.change(themeSelect, { target: { value: 'frost' } });

    expect(document.documentElement.getAttribute('data-color-theme')).toBe('frost');
    expect(localStorage.getItem('auralis:color-theme:v1')).toBe('frost');
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
