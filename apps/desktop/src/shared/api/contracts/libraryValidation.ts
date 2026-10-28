import {
  isRecord,
  isString,
  isNullable,
  isArrayOf,
  isNonNegativeSafeInteger,
  isEnumValue,
} from './validationPrimitives';
const assetKinds = new Set(['text', 'subtitle', 'audio', 'video']);
const toolKinds = new Set(['source', 'text', 'translate', 'speech', 'voices', 'results']);
export const libraryCommandResultValidators = {
  get_tool_context_cmd: (v: unknown) =>
    isRecord(v) &&
    isString(v.projectId) &&
    isString(v.assetId) &&
    ['version', 'artifactId', 'trackId'].every((k) => isNullable(v[k], isString)),
  get_library_cmd: validateLibrary,
  save_library_cmd: validateLibrary,
  read_library_text_cmd: isString,
  get_video_preview_cmd: isString,
  attach_library_asset_cmd: validateLibrary,
  get_media_operation_cmd: (v: unknown) =>
    v === null ||
    (isRecord(v) &&
      ['id', 'projectId', 'assetId', 'status'].every((k) => isString(v[k])) &&
      isNonNegativeSafeInteger(v.completedBytes) &&
      isNonNegativeSafeInteger(v.totalBytes) &&
      isNullable(v.error, isString)),
  cancel_media_operation_cmd: (v: unknown) => v == null,
  validate_media_project_cmd: (v: unknown) => v == null,
  get_media_project_cmd: (v: unknown) =>
    isRecord(v) && v.version === 1 && isString(v.project_id) && Array.isArray(v.assets),
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
        isEnumValue(a.kind, assetKinds) &&
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
            isEnumValue(t.tool, toolKinds) &&
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
