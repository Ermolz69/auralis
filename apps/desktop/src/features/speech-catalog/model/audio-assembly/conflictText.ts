import type { TimingConflict } from '@/shared/api/contracts/audioAssembly';

export function conflictText(conflict: TimingConflict): string {
  switch (conflict.kind) {
    case 'impossible_fit':
      return `${conflict.utterance_id}: речь ${(conflict.required_frames / 48).toFixed(1)} мс, доступно ${(conflict.available_frames / 48).toFixed(1)} мс. Увеличьте окно или выберите непрерывное повествование.`;
    case 'outside_media':
      return `${conflict.utterance_id}: конец окна ${conflict.end_ms} мс выходит за длительность ${conflict.duration_ms} мс.`;
    case 'overlapping_windows':
      return `Окна ${conflict.previous_utterance_id} и ${conflict.utterance_id} пересекаются.`;
    case 'missing_window':
      return `${conflict.utterance_id}: исходное временное окно отсутствует.`;
    case 'invalid_window':
      return `${conflict.utterance_id}: недопустимое окно ${conflict.start_ms}–${conflict.end_ms} мс.`;
    case 'pause_in_subtitle_window':
      return `${conflict.utterance_id}: дополнительные паузы требуют непрерывного режима.`;
    case 'missing_media_duration':
      return 'Укажите длительность отдельной аудиолинии.';
    case 'invalid_count':
      return `Недопустимое число реплик: ${conflict.actual}, предел ${conflict.max}.`;
    case 'coverage_mismatch':
      return 'Порядок или полнота реплик не совпадают с исходной озвучкой.';
    case 'duplicate_utterance':
      return `Повторная реплика ${conflict.utterance_id}.`;
    case 'invalid_audio':
      return `${conflict.utterance_id}: недопустимые параметры аудио.`;
    case 'channel_mismatch':
      return `${conflict.utterance_id}: каналы ${conflict.actual}, ожидается ${conflict.expected}.`;
    case 'riff_size_limit':
      return 'Результат превышает предел поддерживаемого WAV.';
    case 'arithmetic_overflow':
      return 'Длительность или паузы превышают поддерживаемый предел.';
  }
}
