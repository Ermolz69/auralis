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
const voices = ['ciri', 'geralt'].map((voice) => ({
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
  voiceId: 'ciri',
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

it('requires explicit text review and clears that review when text changes', async () => {
  render(<SpeechWorkflow {...props} />);
  fireEvent.click(screen.getByRole('button', { name: /^Текст$/ }));
  const text = screen.getByLabelText('Русский текст для озвучки');
  fireEvent.change(text, { target: { value: 'Проверенный текст.' } });
  expect(screen.getByRole('button', { name: 'Подготовить план' })).toHaveProperty('disabled', true);
  fireEvent.click(screen.getByRole('button', { name: 'Текст проверен' }));
  await screen.findByText('Снимок проверенного текста сохранён.');
  fireEvent.click(screen.getByRole('button', { name: 'Подготовить план' }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith('prepare_speech_plan_cmd', {
      projectId: 'project-1',
      request: {
        decision_id: review.decision_id,
        expected_source_sha256: source.sha256,
        profile_id: props.profileId,
        voice_id: 'ciri',
        reference_id: 'neutral',
        assignments: [],
      },
    }),
  );
  fireEvent.change(text, { target: { value: 'Изменённый текст.' } });
  expect(screen.queryByText('Снимок проверенного текста сохранён.')).toBeNull();
  expect(screen.getByRole('button', { name: 'Подготовить план' })).toHaveProperty('disabled', true);
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
  fireEvent.click(screen.getByRole('button', { name: 'Текст проверен' }));
  fireEvent.change(await screen.findByLabelText('Голос роли Narrator'), {
    target: { value: 'geralt' },
  });
  fireEvent.change(screen.getByLabelText('Голос реплики cue-2'), { target: { value: 'ciri' } });
  fireEvent.click(screen.getByRole('button', { name: 'Подготовить план' }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith('prepare_speech_plan_cmd', {
      projectId: 'project-1',
      request: {
        decision_id: review.decision_id,
        expected_source_sha256: source.sha256,
        profile_id: props.profileId,
        voice_id: 'ciri',
        reference_id: 'neutral',
        assignments: [
          { scope: 'role', target: 'Narrator', voice_id: 'geralt', reference_id: 'neutral' },
          { scope: 'cue', target: 'cue-2', voice_id: 'ciri', reference_id: 'neutral' },
        ],
      },
    }),
  );
});
