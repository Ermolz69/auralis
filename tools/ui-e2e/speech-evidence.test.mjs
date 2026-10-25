import assert from 'node:assert/strict';
import { test } from 'node:test';
import { verifySpeechEvidence } from './speech-evidence.mjs';

const selection = {
  engine_id: 'voxcpm2',
  profile_id: 'profile',
  voice_id: 'voice',
  reference_id: 'neutral',
};
const review = {
  decision_id: 'review',
  project_id: 'project',
  source: {
    kind: 'translation',
    source_id: 'translation',
    sha256: 'a'.repeat(64),
    revision: 'version-1',
    cues: [
      { id: 'cue-1', text: 'Первая реплика.', start_ms: 0, end_ms: 1000 },
      { id: 'cue-2', text: 'Вторая реплика.', start_ms: 1000, end_ms: 2000 },
    ],
  },
};
const plan = {
  project_id: 'project',
  profile: { engine_id: 'voxcpm2', profile_id: 'profile' },
  cast: [{ role_id: 'Narrator', voice_id: 'voice', manifest_sha256: 'b'.repeat(64) }],
  source: { source_sha256: review.source.sha256, text_revision: 'version-1' },
  review: { decision_id: 'review', status: 'approved' },
  utterances: review.source.cues.map((cue, index) => ({
    utterance_id: `utterance-${index}`,
    cue_id: cue.id,
    text: cue.text,
    start_ms: cue.start_ms,
    end_ms: cue.end_ms,
    role_id: 'Narrator',
    reference_id: 'neutral',
  })),
};

test('persisted CONTRACT plans resolve the profile and cast without invented utterance fields', () => {
  assert.doesNotThrow(() => verifySpeechEvidence(plan, review, [selection]));
  for (const key of ['engine_id', 'profile_id', 'voice_id', 'reference_id'])
    assert.throws(() => verifySpeechEvidence(plan, review, [{ ...selection, [key]: 'different' }]));
});
test('missing roles, repeated cues and changed approved text or timing cannot pass evidence checks', () => {
  for (const change of [
    { cast: [] },
    { utterances: [...plan.utterances].reverse() },
    { utterances: [plan.utterances[0], plan.utterances[0]] },
    { utterances: plan.utterances.map((item) => ({ ...item, text: 'Different text' })) },
    { utterances: plan.utterances.map((item) => ({ ...item, end_ms: item.end_ms + 1 })) },
    { review: { decision_id: 'another-review', status: 'approved' } },
  ])
    assert.throws(() => verifySpeechEvidence({ ...plan, ...change }, review, [selection]));
});
test('plain text plans still require the exact reviewed text and revision', () => {
  const source = {
    kind: 'text',
    text: 'Проверенный текст.',
    sha256: 'a'.repeat(64),
    revision: 'version-1',
  };
  const speech = { ...plan, utterances: [{ ...plan.utterances[0], text: source.text }] };
  assert.doesNotThrow(() => verifySpeechEvidence(speech, { ...review, source }, [selection]));
  assert.throws(() =>
    verifySpeechEvidence(speech, { ...review, source: { ...source, revision: 'changed' } }, [
      selection,
    ]),
  );
});
