import { invoke } from '@/shared/api/tauri';
import type { ProjectPin, ProjectPins } from '@/shared/api/contracts/uiPreferences';
import { notifyProjectPreferences, preferencesStorageKey } from './preferencesStorage';

const confirmed = new Map<string, ProjectPin>();
const pending = new Map<string, boolean>();
const attempts = new Map<string, number>();
const writes = new Map<string, Promise<void>>();
let initialized = false;
let loading: Promise<void> | null = null;

export function getStoredPin(projectId: string): boolean | undefined {
  return (
    pending.get(projectId) ??
    (initialized ? (confirmed.get(projectId)?.pinned ?? false) : undefined)
  );
}
export function forgetStoredPin(projectId: string) {
  attempts.set(projectId, (attempts.get(projectId) ?? 0) + 1);
  pending.delete(projectId);
  confirmed.delete(projectId);
}
function accept(pins: ProjectPins) {
  let changed = !initialized;
  for (const pin of pins.entries) {
    if ((confirmed.get(pin.projectId)?.revision ?? 0) < pin.revision) {
      confirmed.set(pin.projectId, pin);
      changed = true;
    }
  }
  initialized = true;
  if (changed) notifyProjectPreferences('*');
}

export function loadProjectPins(): Promise<void> {
  if (loading) return loading;
  loading = (async () => {
    let pins = await invoke('get_project_pins_cmd');
    if (!pins.migrated) {
      const raw = localStorage.getItem(preferencesStorageKey);
      const parsed: unknown = JSON.parse(raw ?? '{}');
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed))
        throw new Error('Invalid legacy preferences');
      const entries = Object.entries(parsed).flatMap(([projectId, value]) => {
        if (!value || typeof value !== 'object' || Array.isArray(value))
          throw new Error('Invalid legacy pin');
        if (!('pinned' in value)) return [];
        if (
          typeof value.pinned !== 'boolean' ||
          !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(projectId)
        )
          throw new Error('Invalid legacy pin');
        return [{ projectId, pinned: value.pinned }];
      });
      pins = await invoke('import_project_pins_cmd', { entries });
      try {
        if (raw !== null && localStorage.getItem(preferencesStorageKey) === raw) {
          const remaining = Object.fromEntries(
            Object.entries(parsed).flatMap(([id, value]) => {
              const rest = Object.fromEntries(
                Object.entries(value as Record<string, unknown>).filter(
                  ([key]) => key !== 'pinned',
                ),
              );
              return Object.keys(rest).length ? [[id, rest]] : [];
            }),
          );
          if (Object.keys(remaining).length)
            localStorage.setItem(preferencesStorageKey, JSON.stringify(remaining));
          else localStorage.removeItem(preferencesStorageKey);
        }
      } catch {
        /* Migration is acknowledged; keep legacy data if cleanup is unavailable. */
      }
    }
    accept(pins);
  })().finally(() => {
    loading = null;
  });
  return loading;
}

export function setProjectPinned(projectId: string, pinned: boolean) {
  const attempt = (attempts.get(projectId) ?? 0) + 1;
  attempts.set(projectId, attempt);
  pending.set(projectId, pinned);
  notifyProjectPreferences(projectId);
  const result = (writes.get(projectId) ?? Promise.resolve()).then(() =>
    savePin(projectId, pinned, attempt),
  );
  const finished = result.then(
    () => undefined,
    () => undefined,
  );
  writes.set(projectId, finished);
  void finished.then(() => {
    if (writes.get(projectId) === finished) writes.delete(projectId);
  });
  return result;
}

async function savePin(projectId: string, pinned: boolean, attempt: number) {
  let persisted = false;
  try {
    const pins = await invoke('get_project_pins_cmd');
    accept(pins);
    const saved = await invoke('set_project_pin_cmd', {
      projectId,
      pinned,
      expectedRevision: confirmed.get(projectId)?.revision ?? 0,
    });
    if (attempts.get(projectId) === attempt) {
      confirmed.set(projectId, saved);
      pending.delete(projectId);
      persisted = true;
    }
  } catch {
    /* Keep the pending preference for an explicit retry. */
  }
  notifyProjectPreferences(projectId);
  return { preferences: { pinned: getStoredPin(projectId) ?? pinned }, persisted };
}
