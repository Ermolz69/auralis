export const preferencesStorageKey = 'auralis.project-preferences.v1';
export const projectPreferencesEvent = 'auralis:project-preferences-changed';

export type StoredPreferences = { pinned: boolean; avatar?: string | null };
type Entries = Record<string, StoredPreferences>;
const pending = new Map<string, StoredPreferences | null>();
let lastReadable: Entries = Object.create(null);

export function readPreferencesStorage() {
  let entries: Entries = Object.create(null);
  let available = true;
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(preferencesStorageKey) ?? '{}');
    if (!raw || typeof raw !== 'object' || Array.isArray(raw))
      throw new Error('Invalid preferences');
    entries = Object.fromEntries(
      Object.entries(raw).flatMap(([id, value]) => {
        if (!value || typeof value !== 'object' || Array.isArray(value)) return [];
        return [
          [
            id,
            {
              pinned: value.pinned === true,
              ...(typeof value.avatar === 'string' ? { avatar: value.avatar } : {}),
            },
          ],
        ];
      }),
    );
  } catch {
    entries = { ...lastReadable };
    available = false;
  }
  if (available) lastReadable = { ...entries };
  for (const [id, value] of pending) {
    if (value === null) delete entries[id];
    else
      Object.defineProperty(entries, id, {
        value,
        configurable: true,
        enumerable: true,
        writable: true,
      });
  }
  return { entries, available };
}

export function mutatePreferences(
  projectId: string,
  update: (current: StoredPreferences) => StoredPreferences | null,
) {
  const { entries, available } = readPreferencesStorage();
  const current = Object.hasOwn(entries, projectId) ? entries[projectId] : { pinned: false };
  const next = update(current);
  pending.set(projectId, next);
  if (next === null) delete entries[projectId];
  else
    Object.defineProperty(entries, projectId, {
      value: next,
      configurable: true,
      enumerable: true,
      writable: true,
    });
  let persisted = false;
  if (available) {
    try {
      localStorage.setItem(preferencesStorageKey, JSON.stringify(entries));
      lastReadable = { ...entries };
      pending.clear();
      persisted = true;
    } catch {
      persisted = false;
    }
  }
  window.dispatchEvent(new CustomEvent(projectPreferencesEvent, { detail: { projectId } }));
  return { persisted };
}
