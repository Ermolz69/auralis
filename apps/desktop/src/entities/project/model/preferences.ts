import { mutatePreferences, readPreferencesStorage } from './preferencesStorage';
export { projectPreferencesEvent } from './preferencesStorage';

export type ProjectPreferences = { pinned: boolean };
export type PreferencesWriteResult = { preferences: ProjectPreferences; persisted: boolean };

export function getProjectPreferences(projectId: string): ProjectPreferences {
  const { entries } = readPreferencesStorage();
  return { pinned: Object.hasOwn(entries, projectId) && entries[projectId].pinned === true };
}

export function updateProjectPreferences(
  projectId: string,
  patch: Partial<ProjectPreferences>,
): PreferencesWriteResult {
  if (patch.pinned === undefined)
    return { preferences: getProjectPreferences(projectId), persisted: true };
  const result = mutatePreferences(projectId, (current) => ({
    ...current,
    pinned: patch.pinned === true,
  }));
  return { preferences: getProjectPreferences(projectId), ...result };
}

export function removeProjectPreferences(projectId: string) {
  return mutatePreferences(projectId, () => null);
}

export function getLegacyProjectAvatar(projectId: string): string | null {
  const { entries } = readPreferencesStorage();
  return Object.hasOwn(entries, projectId) ? (entries[projectId].avatar ?? null) : null;
}

export function removeLegacyProjectAvatar(projectId: string) {
  return mutatePreferences(projectId, (current) => ({ pinned: current.pinned }));
}
