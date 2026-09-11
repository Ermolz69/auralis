import { invoke } from '@/shared/api/tauri';
import type { StoredTheme } from '@/shared/api/contracts/uiPreferences';
import { COLOR_THEME_STORAGE_KEY, readStoredColorTheme, type ColorTheme } from '@/shared/theme';

let confirmed: StoredTheme | null = null;
let pending: Promise<StoredTheme | null> | null = null;
let writes: Promise<void> = Promise.resolve();
const listeners = new Set<(theme: StoredTheme) => void>();

function accept(theme: StoredTheme | null) {
  if (theme && (!confirmed || theme.revision >= confirmed.revision)) {
    confirmed = theme;
    for (const listener of listeners) listener(theme);
  }
  return confirmed;
}

export function subscribeStoredTheme(listener: (theme: StoredTheme) => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function loadStoredTheme(): Promise<StoredTheme | null> {
  if (pending) return pending;
  pending = (async () => {
    const before = confirmed;
    const stored = await invoke('get_color_theme_cmd');
    if (stored) return accept(stored);
    const legacy = readStoredColorTheme();
    if (legacy.status === 'missing') {
      if (confirmed === before) confirmed = null;
      return confirmed;
    }
    if (legacy.status !== 'valid') throw new Error('Legacy theme is unavailable or unrecognized');
    const imported = await invoke('import_color_theme_cmd', { value: legacy.theme });
    try {
      if (localStorage.getItem(COLOR_THEME_STORAGE_KEY) === legacy.theme)
        localStorage.removeItem(COLOR_THEME_STORAGE_KEY);
    } catch {
      /* The durable value is already acknowledged; legacy cleanup can be retried. */
    }
    return accept(imported);
  })().finally(() => {
    pending = null;
  });
  return pending;
}

export function saveStoredTheme(value: ColorTheme): Promise<StoredTheme> {
  const result = writes.then(async () => {
    const current = await invoke('get_color_theme_cmd');
    const stored = await invoke('set_color_theme_cmd', {
      value,
      expectedRevision: current?.revision ?? 0,
    });
    accept(stored);
    return stored;
  });
  writes = result.then(
    () => undefined,
    () => undefined,
  );
  return result;
}
