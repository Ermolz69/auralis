import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { SpeechWorkflow } from './SpeechWorkflow';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ invoke }));
const source = {
  kind: 'text',
  source_id: null,
  revision: 'a'.repeat(64),
  sha256: 'a'.repeat(64),
  text: 'Проверенный текст.',
  cues: [],
};
const review = { decision_id: 'review-1', project_id: 'project-1', source };
const planned = {
  run_id: 'run-1',
  profile_id: 'native-plan-voxcpm2',
  utterance_count: 1,
  chunk_count: 1,
  job: null,
  result: null,
};
const voices = ['marina_elevenlabs_ru', 'alex_analyst_elevenlabs_ru'].map((voice) => ({
  voice_id: voice,
  display_name: voice,
  description: '',
  manifest_sha256: 'b'.repeat(64),
  references: [
    {
      reference_id: 'neutral',
      engine_id: 'voxcpm2',
      emotion_id: 'neutral',
      transcript: 'Референс',
      audio_sha256: 'c'.repeat(64),
      qualification_status: 'qualified' as const,
    },
  ],
}));
const props = {
  projectId: 'project-1',
  profileId: 'native-plan-voxcpm2',
  voiceId: 'marina_elevenlabs_ru',
  referenceId: 'neutral',
  engineId: 'voxcpm2',
  voices,
};
beforeEach(() => {
  vi.clearAllMocks();
  invoke.mockImplementation((command: string) =>
    Promise.resolve(
      command === 'review_speech_text_cmd'
        ? review
        : command === 'prepare_speech_plan_cmd'
          ? planned
          : [],
    ),
  );
});
afterEach(cleanup);

it('requires applying and checking marker output and discards it after a user edit', async () => {
  const original = { ...source, text: 'Два проекта.' };
  const preparation = {
    version: '1.0.0',
    original,
    prepared: { ...source, text: 'Два проверенных проекта.' },
    changed_targets: 1,
    unresolved_targets: 0,
  };
  const previous = invoke.getMockImplementation()!;
  invoke.mockImplementation((command, args) =>
    command === 'prepare_speech_text_cmd'
      ? Promise.resolve(preparation)
      : command === 'review_prepared_speech_text_cmd'
        ? Promise.resolve({ ...review, source: preparation.prepared, original_source: original })
        : previous(command, args),
  );
  render(<SpeechWorkflow {...props} />);
  const text = screen.getByLabelText('Русский текст для озвучки');
  fireEvent.change(text, { target: { value: original.text } });
  fireEvent.click(screen.getByRole('button', { name: 'Подготовить текст' }));
  await screen.findByRole('button', { name: 'Использовать подготовленный текст' });
  expect(screen.getByRole('button', { name: 'Подготовить озвучку' })).toHaveProperty(
    'disabled',
    true,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Использовать подготовленный текст' }));
  expect(text).toHaveProperty('value', preparation.prepared.text);
  fireEvent.click(screen.getByRole('button', { name: 'Текст проверен' }));
  await screen.findByText('Снимок проверенного текста сохранён.');
  expect(invoke).toHaveBeenCalledWith(
    'review_prepared_speech_text_cmd',
    expect.objectContaining({ preparation, confirmed: true }),
  );
  fireEvent.change(text, { target: { value: 'Другой текст.' } });
  expect(screen.queryByText('Снимок проверенного текста сохранён.')).toBeNull();
  expect(screen.queryByText(/Предлагаемый вариант/)).toBeNull();
});

it('requires explicit text review and clears that review when text changes', async () => {
  render(<SpeechWorkflow {...props} />);
  fireEvent.click(screen.getByRole('button', { name: /^Текст$/ }));
  const text = screen.getByLabelText('Русский текст для озвучки');
  fireEvent.change(text, { target: { value: 'Проверенный текст.' } });
  expect(screen.getByRole('button', { name: 'Подготовить озвучку' })).toHaveProperty(
    'disabled',
    true,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Текст проверен' }));
  await screen.findByText('Снимок проверенного текста сохранён.');
  fireEvent.click(screen.getByRole('button', { name: 'Подготовить озвучку' }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith('prepare_speech_plan_cmd', {
      projectId: 'project-1',
      request: {
        decision_id: review.decision_id,
        expected_source_sha256: source.sha256,
        profile_id: props.profileId,
        voice_id: 'marina_elevenlabs_ru',
        reference_id: 'neutral',
        assignments: [],
      },
    }),
  );
  fireEvent.change(text, { target: { value: 'Изменённый текст.' } });
  expect(screen.queryByText('Снимок проверенного текста сохранён.')).toBeNull();
  expect(screen.getByRole('button', { name: 'Подготовить озвучку' })).toHaveProperty(
    'disabled',
    true,
  );
});

it('retains the common voice while sending separate role and cue assignments', async () => {
  const subtitles = {
    ...source,
    kind: 'transcript',
    cues: [
      { id: 'cue-1', role: 'Narrator', text: 'Первая реплика.', start_ms: 1000, end_ms: 2000 },
      { id: 'cue-2', role: 'Narrator', text: 'Вторая реплика.', start_ms: 3000, end_ms: 4000 },
    ],
  };
  invoke.mockImplementation((command: string) =>
    Promise.resolve(
      command === 'review_speech_text_cmd'
        ? { ...review, source: subtitles }
        : command === 'prepare_speech_plan_cmd'
          ? planned
          : [],
    ),
  );
  render(<SpeechWorkflow {...props} />);
  fireEvent.click(screen.getByRole('button', { name: 'Субтитры' }));
  fireEvent.click(screen.getByRole('button', { name: 'Текст проверен' }));
  fireEvent.change(await screen.findByLabelText('Голос роли Narrator'), {
    target: { value: 'alex_analyst_elevenlabs_ru' },
  });
  fireEvent.change(screen.getByLabelText('Голос реплики cue-2'), { target: { value: 'marina_elevenlabs_ru' } });
  fireEvent.click(screen.getByRole('button', { name: 'Подготовить озвучку' }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith('prepare_speech_plan_cmd', {
      projectId: 'project-1',
      request: {
        decision_id: review.decision_id,
        expected_source_sha256: source.sha256,
        profile_id: props.profileId,
        voice_id: 'marina_elevenlabs_ru',
        reference_id: 'neutral',
        assignments: [
          { scope: 'role', target: 'Narrator', voice_id: 'alex_analyst_elevenlabs_ru', reference_id: 'neutral' },
          { scope: 'cue', target: 'cue-2', voice_id: 'marina_elevenlabs_ru', reference_id: 'neutral' },
        ],
      },
    }),
  );
});
