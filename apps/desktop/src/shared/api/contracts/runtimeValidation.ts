import type { Artifact } from './artifact';
import type { CommandMap, EventMap } from './commandMap';
import { validateJobDto, validateJobEventDto, validateJobSnapshot } from './jobValidation';
import type { Project } from './project';
import { validateMediaMetadata, validateMediaSource } from './mediaValidation';
import {
  isArrayOf,
  isBoolean,
  isEnumValue,
  isNonNegativeSafeInteger,
  isNull,
  isNullable,
  isRecord,
  isString,
  isTimestamp,
  type Validator,
} from './validationPrimitives';
import { isSafeRevision } from './jobValidation';

type CommandResultValidators = { [K in keyof CommandMap]: Validator };
type EventPayloadValidators = { [K in keyof EventMap]: Validator };

const projectStatuses = new Set([
  'draft',
  'source_imported',
  'ready_for_processing',
  'processing',
  'completed',
  'failed',
  'cancelled',
]);
const artifactKinds = new Set([
  'sourceVideo',
  'downloadedVideo',
  'extractedAudio',
  'originalSubtitle',
  'generatedTranscript',
  'normalizedTranscript',
  'translatedTranscript',
  'generatedSpeechSegment',
  'mixedAudio',
  'previewVideo',
  'finalVideo',
]);
const artifactStates = new Set(['pendingFinalize', 'ready', 'deleting', 'failed']);
const pendingImportStates = new Set(['Downloading', 'Staged', 'Failed']);

export class IpcContractError extends Error {
  constructor(kind: 'command' | 'event', name: string) {
    super(`Invalid payload received for IPC ${kind} "${name}"`);
    this.name = 'IpcContractError';
  }
}

export function validateProject(value: unknown): value is Project {
  if (!isRecord(value)) return false;
  return (
    isNonNegativeSafeInteger(value.revision) &&
    value.revision > 0 &&
    isString(value.id) &&
    isString(value.title) &&
    isEnumValue(value.status, projectStatuses) &&
    isTimestamp(value.createdAt) &&
    isTimestamp(value.updatedAt) &&
    isNullable(value.source, validateMediaSource) &&
    isNullable(value.metadata, validateMediaMetadata)
  );
}

function validateStoredTheme(value: unknown): boolean {
  return isRecord(value) && isString(value.value) && isSafeRevision(value.revision);
}
function validatePin(value: unknown): boolean {
  return (
    isRecord(value) &&
    isString(value.projectId) &&
    typeof value.pinned === 'boolean' &&
    isSafeRevision(value.revision)
  );
}
function validatePins(value: unknown): boolean {
  return (
    isRecord(value) && typeof value.migrated === 'boolean' && isArrayOf(value.entries, validatePin)
  );
}

export function validateArtifact(value: unknown): value is Artifact {
  if (!isRecord(value) || !isRecord(value.location)) return false;
  const location = value.location;
  return (
    isString(value.id) &&
    isEnumValue(value.kind, artifactKinds) &&
    (location.kind === 'localPath' || location.kind === 'storageKey') &&
    isString(location.value) &&
    isNullable(value.sizeBytes, isNonNegativeSafeInteger) &&
    isEnumValue(value.state, artifactStates) &&
    isTimestamp(value.createdAt) &&
    isTimestamp(value.updatedAt) &&
    isNullable(value.readyAt, isTimestamp)
  );
}

const commandResultValidators = {
  list_pending_youtube_imports_cmd: (value) =>
    isArrayOf(value, (entry) =>
      isRecord(entry)
        ? isString(entry.projectId) &&
          isString(entry.title) &&
          isEnumValue(entry.state, pendingImportStates)
        : false,
    ),
  resume_youtube_import_cmd: validateProject,
  get_color_theme_cmd: (value) => isNullable(value, validateStoredTheme),
  set_color_theme_cmd: validateStoredTheme,
  import_color_theme_cmd: validateStoredTheme,
  get_project_pins_cmd: validatePins,
  import_project_pins_cmd: validatePins,
  set_project_pin_cmd: validatePin,
  list_artifact_recovery_cmd: (value) => isArrayOf(value, isString),
  retry_artifact_finalization_cmd: isNull,
  discard_youtube_import_cmd: isNull,
  get_project_avatar_cmd: validateProjectAvatar,
  set_project_avatar_cmd: validateProjectAvatar,
  health_check: isString,
  native_e2e_checkpoint_cmd: isNull,
  native_e2e_pipeline_pause_reached_cmd: isBoolean,
  create_project_cmd: validateProject,
  create_project_from_youtube_cmd: validateProject,
  rename_project_cmd: validateProject,
  open_project_folder_cmd: isNull,
  start_project_mock_pipeline_cmd: (value) =>
    isRecord(value) && validateProject(value.project) && validateJobDto(value.job),
  list_youtube_subtitle_tracks_cmd: (value) => isArrayOf(value, validateSubtitleTrack),
  get_transcript_cmd: (value) => isNullable(value, validateTranscript),
  get_project_cmd: validateProject,
  list_projects_cmd: (value) => isArrayOf(value, validateProject),
  delete_project_cmd: isNull,
  list_project_artifacts_cmd: (value) => isArrayOf(value, validateArtifact),
  resolve_artifact_path_cmd: isString,
  list_jobs_cmd: validateJobSnapshot,
  list_job_history_page_cmd: validateJobHistoryPage,
  list_jobs_snapshot_cmd: validateJobSnapshot,
  cancel_job_cmd: validateJobDto,
  probe_local_media_cmd: validateMediaMetadata,
  import_local_media_cmd: validateProject,
} satisfies CommandResultValidators;

const eventPayloadValidators = {
  'job-event': validateJobEventDto,
  'job-events-invalidated': isNull,
  'project-updated': validateProjectIdPayload,
  'transcript-ready': (value) =>
    validateProjectIdPayload(value) && isString((value as Record<string, unknown>).jobId),
} satisfies EventPayloadValidators;

function validateJobHistoryPage(value: unknown): boolean {
  if (!isRecord(value) || !validateJobSnapshot(value.jobs)) return false;
  return isNullable(value.nextCursor, (cursor) => {
    return isRecord(cursor) && isTimestamp(cursor.createdAt) && isString(cursor.jobId);
  });
}

export function parseCommandResult<K extends keyof CommandMap>(
  command: K,
  value: unknown,
): CommandMap[K]['result'] {
  if (!commandResultValidators[command](value)) throw new IpcContractError('command', command);
  return value as CommandMap[K]['result'];
}

export function parseEventPayload<K extends keyof EventMap>(event: K, value: unknown): EventMap[K] {
  if (!eventPayloadValidators[event](value)) throw new IpcContractError('event', event);
  return value as EventMap[K];
}

function validateProjectAvatar(value: unknown): boolean {
  return (
    isRecord(value) && isNullable(value.dataUrl, isString) && typeof value.initialized === 'boolean'
  );
}

function validateSubtitleTrack(value: unknown): boolean {
  return (
    isRecord(value) &&
    isString(value.id) &&
    isString(value.language) &&
    isNullable(value.label, isString) &&
    isNullable(value.format, isString) &&
    typeof value.isAutoGenerated === 'boolean'
  );
}

function validateTranscript(value: unknown): boolean {
  if (!isRecord(value) || !isString(value.language) || !Array.isArray(value.segments)) return false;

  const ids = new Set<string>();
  const indexes = new Set<number>();
  for (const segment of value.segments) {
    if (
      !isRecord(segment) ||
      !isString(segment.id) ||
      !isNonNegativeSafeInteger(segment.index) ||
      !isNonNegativeSafeInteger(segment.startMs) ||
      !isNonNegativeSafeInteger(segment.endMs) ||
      segment.endMs < segment.startMs ||
      !isString(segment.sourceText) ||
      ids.has(segment.id) ||
      indexes.has(segment.index)
    ) {
      return false;
    }
    ids.add(segment.id);
    indexes.add(segment.index);
  }
  return true;
}

function validateProjectIdPayload(value: unknown): boolean {
  return isRecord(value) && isString(value.projectId);
}
