// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke }));
const key = 'auralis.project-preferences.v1';
const id = 'a1111111-1111-4111-8111-111111111111';
beforeEach(() => {
  vi.resetModules();
  invoke.mockReset();
  localStorage.clear();
});

describe('native pin preferences', () => {
  it('orders two observers writes even when the first read is delayed', async () => {
    let record = { projectId: id, pinned: false, revision: 1 };
    invoke.mockImplementation(
      async (command: string, args?: { pinned: boolean; expectedRevision: number }) => {
        if (command === 'get_project_pins_cmd') return { migrated: true, entries: [record] };
        expect(args!.expectedRevision).toBe(record.revision);
        record = { ...record, pinned: args!.pinned, revision: record.revision + 1 };
        return record;
      },
    );
    let resume!: (value: unknown) => void;
    invoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resume = resolve;
        }),
    );
    const pins = await import('./pinPersistence');
    const older = pins.setProjectPinned(id, true);
    await vi.waitFor(() => expect(resume).toBeDefined());
    const newer = pins.setProjectPinned(id, false);
    expect(pins.getStoredPin(id)).toBe(false);
    resume({ migrated: true, entries: [record] });
    await older;
    expect((await newer).persisted).toBe(true);
    expect(record.pinned).toBe(false);
    expect(record.revision).toBe(3);
  });
  it('removes only acknowledged legacy pins and retains avatar data', async () => {
    const raw = JSON.stringify({ [id]: { pinned: true, avatarDataUrl: 'avatar' } });
    localStorage.setItem(key, raw);
    let acknowledge!: (value: unknown) => void;
    invoke.mockResolvedValueOnce({ migrated: false, entries: [] }).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          acknowledge = resolve;
        }),
    );
    const pins = await import('./pinPersistence');
    const loading = pins.loadProjectPins();
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
    expect(localStorage.getItem(key)).toBe(raw);
    acknowledge({ migrated: true, entries: [{ projectId: id, pinned: true, revision: 1 }] });
    await loading;
    expect(JSON.parse(localStorage.getItem(key)!)).toEqual({ [id]: { avatarDataUrl: 'avatar' } });
    expect(pins.getStoredPin(id)).toBe(true);
  });

  it('keeps failed migration retryable and does not overwrite malformed legacy data', async () => {
    localStorage.setItem(key, '{broken');
    invoke.mockResolvedValue({ migrated: false, entries: [] });
    const pins = await import('./pinPersistence');
    await expect(pins.loadProjectPins()).rejects.toThrow();
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(localStorage.getItem(key)).toBe('{broken');
    const raw = JSON.stringify({ [id]: { pinned: true } });
    localStorage.setItem(key, raw);
    invoke
      .mockResolvedValueOnce({ migrated: false, entries: [] })
      .mockRejectedValueOnce(new Error('disk'));
    await expect(pins.loadProjectPins()).rejects.toThrow('disk');
    expect(localStorage.getItem(key)).toBe(raw);
    invoke.mockResolvedValueOnce({ migrated: false, entries: [] }).mockResolvedValueOnce({
      migrated: true,
      entries: [{ projectId: id, pinned: true, revision: 1 }],
    });
    await pins.loadProjectPins();
    expect(localStorage.getItem(key)).toBeNull();
  });

  it('uses native values after WebView clearing without importing stale legacy pins', async () => {
    localStorage.setItem(key, JSON.stringify({ [id]: { pinned: false } }));
    invoke.mockResolvedValue({
      migrated: true,
      entries: [{ projectId: id, pinned: true, revision: 2 }],
    });
    const pins = await import('./pinPersistence');
    await pins.loadProjectPins();
    expect(pins.getStoredPin(id)).toBe(true);
    localStorage.clear();
    vi.resetModules();
    const reopened = await import('./pinPersistence');
    await reopened.loadProjectPins();
    expect(reopened.getStoredPin(id)).toBe(true);
    expect(invoke.mock.calls.every(([command]) => command === 'get_project_pins_cmd')).toBe(true);
  });

  it('retains failed intent for retry and ignores an acknowledgement after removal', async () => {
    invoke
      .mockResolvedValueOnce({ migrated: true, entries: [] })
      .mockRejectedValueOnce(new Error('disk'));
    const pins = await import('./pinPersistence');
    expect((await pins.setProjectPinned(id, true)).persisted).toBe(false);
    expect(pins.getStoredPin(id)).toBe(true);
    let acknowledge!: (value: unknown) => void;
    invoke.mockResolvedValueOnce({ migrated: true, entries: [] }).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          acknowledge = resolve;
        }),
    );
    const retry = pins.setProjectPinned(id, true);
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(4));
    pins.forgetStoredPin(id);
    acknowledge({ projectId: id, pinned: true, revision: 1 });
    expect((await retry).persisted).toBe(false);
    expect(pins.getStoredPin(id)).toBe(false);
  });
});
