import type { MediaMetadata, MediaSource } from './media';
import {
  isRecord,
  isString,
  isNonNegativeSafeInteger,
  nullableField,
  isPositiveFiniteNumber,
} from './validationPrimitives';

export function validateMediaMetadata(value: unknown): value is MediaMetadata {
  if (!isRecord(value)) return false;
  return (
    isNonNegativeSafeInteger(value.durationMs) &&
    nullableField(value, 'width', isNonNegativeSafeInteger) &&
    nullableField(value, 'height', isNonNegativeSafeInteger) &&
    nullableField(value, 'fps', isPositiveFiniteNumber) &&
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

export function validateMediaSource(value: unknown): value is MediaSource {
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
    nullableField(value, 'fps', isPositiveFiniteNumber) &&
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
