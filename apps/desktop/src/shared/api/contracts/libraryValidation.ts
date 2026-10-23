import {
  isRecord,
  isString,
  isNullable,
  isArrayOf,
  isNonNegativeSafeInteger,
} from './validationPrimitives';
export const libraryCommandResultValidators = {
  get_library_cmd: validateLibrary,
  save_library_cmd: validateLibrary,
  read_library_text_cmd: isString,
};
const stringOrNull = (value: unknown) => isNullable(value, isString);
export function validateLibrary(value: unknown): boolean {
  if (!isRecord(value) || !isNonNegativeSafeInteger(value.revision) || !isRecord(value.browser))
    return false;
  const strings = (v: unknown) => isArrayOf(v, isString);
  const browser = value.browser;
  return (
    isArrayOf(
      value.assets,
      (a) =>
        isRecord(a) &&
        isString(a.id) &&
        isString(a.ownerProjectId) &&
        isString(a.name) &&
        ['text', 'subtitle', 'audio', 'video'].includes(String(a.kind)) &&
        ['artifactId', 'format', 'text', 'parentId', 'openedAt'].every((k) => stringOrNull(a[k])) &&
        strings(a.tags),
    ) &&
    isArrayOf(
      value.folders,
      (f) =>
        isRecord(f) &&
        isString(f.id) &&
        isString(f.projectId) &&
        isString(f.name) &&
        stringOrNull(f.parentId),
    ) &&
    isArrayOf(
      value.links,
      (l) =>
        isRecord(l) && isString(l.assetId) && isString(l.projectId) && stringOrNull(l.folderId),
    ) &&
    strings(value.standaloneOwners) &&
    strings(value.removedProjects) &&
    strings(value.hiddenAssets) &&
    isArrayOf(
      value.workspaces,
      (w) =>
        isRecord(w) &&
        isString(w.id) &&
        stringOrNull(w.projectId) &&
        stringOrNull(w.assetId) &&
        stringOrNull(w.activeTab) &&
        isArrayOf(
          w.tabs,
          (t) =>
            isRecord(t) &&
            isString(t.id) &&
            ['source', 'text', 'translate', 'speech', 'voices', 'results'].includes(
              String(t.tool),
            ) &&
            stringOrNull(t.assetId) &&
            isString(t.draft),
        ),
    ) &&
    isString(value.browser.scope) &&
    isString(value.browser.search) &&
    isString(value.browser.kind) &&
    ['projectId', 'folderId', 'selectedId'].every((k) => stringOrNull(browser[k]))
  );
}
