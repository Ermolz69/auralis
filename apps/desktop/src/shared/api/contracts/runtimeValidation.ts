import type { Artifact } from './artifact';
import type { CommandMap, EventMap } from './commandMap';
import { validateJobDto, validateJobEventDto, validateJobSnapshot } from './jobValidation';
import type { MediaMetadata, MediaSource } from './media';
import type { Project } from './project';

type Validator = (value: unknown) => boolean;
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
    isString(value.id) &&
    isString(value.title) &&
    isEnumValue(value.status, projectStatuses) &&
    isTimestamp(value.createdAt) &&
    isTimestamp(value.updatedAt) &&
    isNullable(value.source, validateMediaSource) &&
    isNullable(value.metadata, validateMediaMetadata)
  );
}

export function validateMediaMetadata(value: unknown): value is MediaMetadata {
  if (!isRecord(value)) return false;
  return (
    isNonNegativeSafeInteger(value.durationMs) &&
    nullableField(value, 'width', isNonNegativeSafeInteger) &&
    nullableField(value, 'height', isNonNegativeSafeInteger) &&
    nullableField(value, 'fps', isFiniteNumber) &&
    nullableField(value, 'videoCodec', isString) &&
    nullableField(value, 'audioCodec', isString) &&
    nullableField(value, 'sampleRate', isNonNegativeSafeInteger) &&
    nullableField(value, 'audioChannels', isNonNegativeSafeInteger) &&
    nullableField(value, 'container', isString) &&
    nullableField(value, 'bitrate', isNonNegativeSafeInteger) &&
    nullableField(value, 'formatName', isString) &&
    typeof value.hasVideo === 'boolean' &&
    typeof value.hasAudio === 'boolean' &&
    Array.isArray(value.streams) &&
    value.streams.every(validateMediaStream) &&
    nullableField(value, 'video', validateVideoStream) &&
    Array.isArray(value.audioTracks) &&
    value.audioTracks.every(validateAudioTrack)
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
  discard_youtube_import_cmd: isNull,
  get_project_avatar_cmd: validateProjectAvatar,
  set_project_avatar_cmd: validateProjectAvatar,
  health_check: isString,
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

function validateMediaSource(value: unknown): value is MediaSource {
  if (!isRecord(value) || !isString(value.kind)) return false;
  switch (value.kind) {
    case 'managedLocalFile':
      return isString(value.artifactId) && isString(value.originalFilename);
    case 'youtubeUrl':
    case 'remoteUrl':
      return isString(value.url);
    case 'externalLocalFile':
      return isString(value.path);
    default:
      return false;
  }
}

function validateMediaStream(value: unknown): boolean {
  return (
    isRecord(value) &&
    isNonNegativeSafeInteger(value.index) &&
    isString(value.codecType) &&
    nullableField(value, 'codecName', isString) &&
    nullableField(value, 'codecLongName', isString) &&
    nullableField(value, 'language', isString) &&
    nullableField(value, 'durationMs', isNonNegativeSafeInteger)
  );
}

function validateVideoStream(value: unknown): boolean {
  return (
    isRecord(value) &&
    isNonNegativeSafeInteger(value.streamIndex) &&
    isNonNegativeSafeInteger(value.width) &&
    isNonNegativeSafeInteger(value.height) &&
    nullableField(value, 'fps', isFiniteNumber) &&
    nullableField(value, 'codec', isString) &&
    nullableField(value, 'pixelFormat', isString)
  );
}

function validateAudioTrack(value: unknown): boolean {
  return (
    isRecord(value) &&
    isNonNegativeSafeInteger(value.streamIndex) &&
    nullableField(value, 'codec', isString) &&
    nullableField(value, 'channels', isNonNegativeSafeInteger) &&
    nullableField(value, 'channelLayout', isString) &&
    nullableField(value, 'sampleRate', isNonNegativeSafeInteger) &&
    nullableField(value, 'language', isString) &&
    nullableField(value, 'title', isString) &&
    typeof value.isDefault === 'boolean'
  );
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
  return (
    isRecord(value) &&
    isString(value.language) &&
    isArrayOf(value.segments, (segment) =>
      isRecord(segment)
        ? isString(segment.id) &&
          isNonNegativeSafeInteger(segment.index) &&
          isNonNegativeSafeInteger(segment.startMs) &&
          isNonNegativeSafeInteger(segment.endMs) &&
          isString(segment.sourceText)
        : false,
    )
  );
}

function validateProjectIdPayload(value: unknown): boolean {
  return isRecord(value) && isString(value.projectId);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === 'string';
}

function isNull(value: unknown): value is null {
  return value === null;
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function isNonNegativeSafeInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

function isTimestamp(value: unknown): value is string {
  return isString(value) && Number.isFinite(Date.parse(value));
}

function isEnumValue(value: unknown, values: ReadonlySet<string>): value is string {
  return isString(value) && values.has(value);
}

function isNullable(value: unknown, validator: Validator): boolean {
  return value === null || validator(value);
}

function nullableField(
  record: Record<string, unknown>,
  key: string,
  validator: Validator,
): boolean {
  return !Object.hasOwn(record, key) || record[key] === null || validator(record[key]);
}

function isArrayOf(value: unknown, validator: Validator): boolean {
  return Array.isArray(value) && value.every(validator);
}
