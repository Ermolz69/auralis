// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const key = 'auralis.project-preferences.v1';
beforeEach(() => {
  vi.resetModules();
  localStorage.clear();
});
afterEach(() => vi.restoreAllMocks());

describe('project preferences persistence', () => {
  it.each(['QuotaExceededError', 'SecurityError'])(
    'keeps mutations in memory when storage throws %s',
    async (name) => {
      const prefs = await import('./preferences');
      const spy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
        throw new DOMException('blocked', name);
      });
      expect(prefs.updateProjectPreferences('p', { pinned: true })).toEqual({
        preferences: { pinned: true },
        persisted: false,
      });
      expect(prefs.getProjectPreferences('p').pinned).toBe(true);
      expect(prefs.removeProjectPreferences('p').persisted).toBe(false);
      expect(prefs.getProjectPreferences('p').pinned).toBe(false);
      spy.mockRestore();
      expect(prefs.updateProjectPreferences('other', { pinned: true }).persisted).toBe(true);
      expect(JSON.parse(localStorage.getItem(key)!)).toEqual({ other: { pinned: true } });
    },
  );

  it('does not overwrite unreadable storage and preserves known preferences', async () => {
    const prefs = await import('./preferences');
    prefs.updateProjectPreferences('known', { pinned: true });
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new DOMException('blocked', 'SecurityError');
    });
    const write = vi.spyOn(Storage.prototype, 'setItem');
    expect(prefs.getProjectPreferences('known').pinned).toBe(true);
    expect(prefs.updateProjectPreferences('p', { pinned: true }).persisted).toBe(false);
    expect(write).not.toHaveBeenCalled();
  });

  it.each(['null', '[]', '42', '"text"', '{bad'])(
    'tolerates malformed stored JSON %s',
    async (value) => {
      localStorage.setItem(key, value);
      const prefs = await import('./preferences');
      expect(prefs.getProjectPreferences('p')).toEqual({ pinned: false });
      expect(prefs.updateProjectPreferences('p', { pinned: true }).persisted).toBe(false);
      expect(localStorage.getItem(key)).toBe(value);
    },
  );

  it('validates entry shapes, preserves legacy avatars until migration and never adds new ones', async () => {
    localStorage.setItem(
      key,
      JSON.stringify({
        legacy: { pinned: true, avatar: 'old-data' },
        invalid: { pinned: 'true' },
        empty: null,
      }),
    );
    const prefs = await import('./preferences');
    expect(prefs.getProjectPreferences('invalid')).toEqual({ pinned: false });
    expect(prefs.getLegacyProjectAvatar('legacy')).toBe('old-data');
    prefs.updateProjectPreferences('p', { pinned: true });
    expect(JSON.parse(localStorage.getItem(key)!).legacy.avatar).toBe('old-data');
    prefs.removeLegacyProjectAvatar('legacy');
    expect(JSON.parse(localStorage.getItem(key)!).legacy).toEqual({ pinned: true });
  });

  it('does not use empty patches as a notification bus', async () => {
    const prefs = await import('./preferences');
    const write = vi.spyOn(Storage.prototype, 'setItem');
    const event = vi.fn();
    window.addEventListener(prefs.projectPreferencesEvent, event);
    prefs.updateProjectPreferences('p', {});
    expect(write).not.toHaveBeenCalled();
    expect(event).not.toHaveBeenCalled();
    window.removeEventListener(prefs.projectPreferencesEvent, event);
  });
});
