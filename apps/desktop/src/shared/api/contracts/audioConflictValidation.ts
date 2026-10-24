import { isNonNegativeSafeInteger as integer, isRecord, isString } from './validationPrimitives';

export function validateTimingConflict(value: unknown): boolean {
  if (!isRecord(value)) return false;
  const id = () => isString(value.utterance_id) && value.utterance_id.length > 0;
  switch (value.kind) {
    case 'coverage_mismatch':
    case 'missing_media_duration':
    case 'arithmetic_overflow':
      return true;
    case 'duplicate_utterance':
    case 'invalid_audio':
    case 'missing_window':
    case 'pause_in_subtitle_window':
      return id();
    case 'invalid_count':
      return integer(value.actual) && integer(value.max);
    case 'channel_mismatch':
      return id() && integer(value.expected) && integer(value.actual);
    case 'invalid_window':
      return id() && integer(value.start_ms) && integer(value.end_ms);
    case 'outside_media':
      return id() && integer(value.end_ms) && integer(value.duration_ms);
    case 'overlapping_windows':
      return (
        id() && isString(value.previous_utterance_id) && value.previous_utterance_id.length > 0
      );
    case 'impossible_fit':
      return id() && integer(value.required_frames) && integer(value.available_frames);
    case 'riff_size_limit':
      return integer(value.frames) && integer(value.channels);
    default:
      return false;
  }
}
