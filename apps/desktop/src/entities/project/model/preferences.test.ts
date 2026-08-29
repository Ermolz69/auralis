// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  getProjectPreferences,
  projectPreferencesEvent,
  removeProjectPreferences,
  updateProjectPreferences,
} from './preferences';

const storageKey = 'auralis.project-preferences.v1';

describe('project preferences', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns defaults when preferences have not been saved', () => {
    expect(getProjectPreferences('project-1')).toEqual({ pinned: false, avatar: null });
  });

  it('falls back to defaults when stored preferences are corrupted', () => {
    localStorage.setItem(storageKey, '{not valid json');

    expect(getProjectPreferences('project-1')).toEqual({ pinned: false, avatar: null });
  });

  it.each([
    ['null', 'null'],
    ['an array', '[]'],
  ])('falls back to defaults when storage contains %s', (_label, value) => {
    localStorage.setItem(storageKey, value);

    expect(getProjectPreferences('project-1')).toEqual({ pinned: false, avatar: null });
  });

  it('does not dispatch a change event when localStorage quota is exceeded', () => {
    const listener = vi.fn();
    window.addEventListener(projectPreferencesEvent, listener);
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new DOMException('Quota exceeded', 'QuotaExceededError');
    });

    expect(() => updateProjectPreferences('project-1', { pinned: true })).toThrow('Quota exceeded');
    expect(listener).not.toHaveBeenCalled();
    window.removeEventListener(projectPreferencesEvent, listener);
  });

  it('merges updates and preserves preferences for other projects', () => {
    updateProjectPreferences('project-1', { pinned: true });
    updateProjectPreferences('project-1', { avatar: 'data:image/png;base64,avatar' });
    updateProjectPreferences('project-2', { avatar: 'data:image/png;base64,other' });

    expect(getProjectPreferences('project-1')).toEqual({
      pinned: true,
      avatar: 'data:image/png;base64,avatar',
    });
    expect(getProjectPreferences('project-2')).toEqual({
      pinned: false,
      avatar: 'data:image/png;base64,other',
    });
  });

  it('dispatches a project-scoped change event after an update', () => {
    const listener = vi.fn();
    window.addEventListener(projectPreferencesEvent, listener);

    updateProjectPreferences('project-1', { pinned: true });

    expect(listener).toHaveBeenCalledOnce();
    expect((listener.mock.calls[0][0] as CustomEvent).detail).toEqual({ projectId: 'project-1' });
    window.removeEventListener(projectPreferencesEvent, listener);
  });

  it('removes only the selected project and dispatches a change event', () => {
    updateProjectPreferences('project-1', { pinned: true });
    updateProjectPreferences('project-2', { pinned: true });
    const listener = vi.fn();
    window.addEventListener(projectPreferencesEvent, listener);

    removeProjectPreferences('project-1');

    expect(getProjectPreferences('project-1')).toEqual({ pinned: false, avatar: null });
    expect(getProjectPreferences('project-2')).toEqual({ pinned: true, avatar: null });
    expect(listener).toHaveBeenCalledOnce();
    expect((listener.mock.calls[0][0] as CustomEvent).detail).toEqual({ projectId: 'project-1' });
    window.removeEventListener(projectPreferencesEvent, listener);
  });
});
