import assert from 'node:assert/strict';
import { test } from 'node:test';
import { verifyRawLayout } from './layout.mjs';
const window = { start_ms: 100, end_ms: 1000 };
const speech = { utterances: [{ utterance_id: 'u', ...window }] };
const plan = {
  raw_wav: { sha256: 'wav' },
  raw_manifest: { sha256: 'manifest' },
  timeline: {
    source_plan_sha256: 'plan',
    raw_manifest_sha256: 'manifest',
    utterances: [{ frame_count: 48000, sample_rate: 48000, channels: 1, source_window: window }],
  },
};
const layout = {
  aggregate_sha256: 'wav',
  source_plan_sha256: 'plan',
  raw_manifest_sha256: 'manifest',
  assembly_policy: 'pcm16_stream_fade10ms_peak089125_kaiser5_v1',
  sample_rate: 48000,
  channels: 1,
  frame_count: 48000,
  utterances: [
    {
      utterance_id: 'u',
      first_frame: 0,
      frame_count: 48000,
      chunk_ids: ['c1', 'c2'],
      source_window: window,
    },
  ],
};
const manifest = {
  output_sha256: 'wav',
  audio_assembly: layout.assembly_policy,
  sample_rate: 48000,
  segments: [
    { utterance_id: 'u', chunk_id: 'c1' },
    { utterance_id: 'u', chunk_id: 'c2' },
  ],
};
const chunks = [
  { frames: 12000, sample_rate: 24000, channels: 1 },
  { frames: 24000, sample_rate: 48000, channels: 1 },
];
test('complete chunk metadata cannot hide a shortened raw WAV or altered layout bindings', () => {
  verifyRawLayout(plan, 'plan', speech, layout, manifest, { frames: 48000, channels: 1 }, chunks);
  assert.throws(() =>
    verifyRawLayout(plan, 'plan', speech, layout, manifest, { frames: 24000, channels: 1 }, chunks),
  );
  for (const change of [
    { aggregate_sha256: 'other' },
    { raw_manifest_sha256: 'other' },
    { source_plan_sha256: 'other' },
    { frame_count: 24000, utterances: [{ ...layout.utterances[0], frame_count: 24000 }] },
  ])
    assert.throws(() =>
      verifyRawLayout(
        plan,
        'plan',
        speech,
        { ...layout, ...change },
        manifest,
        { frames: 48000, channels: 1 },
        chunks,
      ),
    );
});
