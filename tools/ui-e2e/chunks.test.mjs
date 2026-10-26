import assert from 'node:assert/strict';
import { test } from 'node:test';
import { verifyChunks } from './chunks.mjs';
const plan = {
  plan_id: 'p',
  plan_digest: 'digest',
  profile: { engine_id: 'voxcpm2', profile_id: 'profile', model_revision: 'model' },
  cast: [{ role_id: 'n', voice_id: 'v' }],
  utterances: [
    {
      utterance_id: 'u',
      cue_id: 'cue-1',
      start_ms: 0,
      end_ms: 100,
      text: 'Hello world.',
      role_id: 'n',
      reference_id: 'r',
    },
  ],
};
const manifest = {
  version: 4,
  engine: 'voxcpm2',
  profile: 'profile',
  speech_plan: { plan_id: 'p', plan_digest: 'digest', expected_utterance_ids: ['u'] },
  runtime: { model_revisions: { 'openbmb/VoxCPM2': 'model' } },
  segments: [
    {
      utterance_id: 'u',
      chunk_id: 'u-chunk-0001',
      cue_id: 'cue-1',
      source_start_ms: 0,
      source_end_ms: 100,
      resolved_voice: 'v',
      reference_id: 'r',
      engine: 'voxcpm2',
      status: 'generated',
      duration_ms: 20,
      text: 'Hello ',
    },
    {
      utterance_id: 'u',
      chunk_id: 'u-chunk-0002',
      cue_id: 'cue-1',
      source_start_ms: 0,
      source_end_ms: 100,
      resolved_voice: 'v',
      reference_id: 'r',
      engine: 'voxcpm2',
      status: 'cached',
      duration_ms: 20,
      text: 'world.',
    },
  ],
};
test('final chunks preserve complete ordered text and timing independently of ready publication', () => {
  assert.deepEqual(verifyChunks(plan, manifest), { utterances: 1, chunks: 2 });
  for (const change of [
    { segments: manifest.segments.slice(0, 1) },
    { segments: [...manifest.segments].reverse() },
    { segments: manifest.segments.map((s) => ({ ...s, source_end_ms: 99 })) },
    { segments: manifest.segments.map((s) => ({ ...s, resolved_voice: 'other' })) },
  ])
    assert.throws(() => verifyChunks(plan, { ...manifest, ...change }));
});
