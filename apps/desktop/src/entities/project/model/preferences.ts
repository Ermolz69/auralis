const storageKey = 'auralis.project-preferences.v1';
export const projectPreferencesEvent = 'auralis:project-preferences-changed';

export type ProjectPreferences = {
  pinned: boolean;
  avatar: string | null;
};

type StoredPreferences = Record<string, ProjectPreferences>;

function readAll(): StoredPreferences {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(storageKey) ?? '{}');
    if (!isRecord(parsed)) return {};

    return Object.fromEntries(
      Object.entries(parsed).filter((entry): entry is [string, ProjectPreferences] =>
        isProjectPreferences(entry[1]),
      ),
    );
  } catch {
    return {};
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isProjectPreferences(value: unknown): value is ProjectPreferences {
  return (
    isRecord(value) &&
    typeof value.pinned === 'boolean' &&
    (typeof value.avatar === 'string' || value.avatar === null)
  );
}

export function getProjectPreferences(projectId: string): ProjectPreferences {
  return readAll()[projectId] ?? { pinned: false, avatar: null };
}

export function updateProjectPreferences(
  projectId: string,
  patch: Partial<ProjectPreferences>,
): ProjectPreferences {
  const all = readAll();
  const next = { ...getProjectPreferences(projectId), ...patch };
  all[projectId] = next;
  localStorage.setItem(storageKey, JSON.stringify(all));
  window.dispatchEvent(new CustomEvent(projectPreferencesEvent, { detail: { projectId } }));
  return next;
}

export function removeProjectPreferences(projectId: string) {
  const all = readAll();
  delete all[projectId];
  localStorage.setItem(storageKey, JSON.stringify(all));
  window.dispatchEvent(new CustomEvent(projectPreferencesEvent, { detail: { projectId } }));
}
