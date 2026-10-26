import assert from 'node:assert/strict';

export function verifyChunks(plan, manifest) {
  assert.equal(manifest.version, 4);
  assert.equal(manifest.engine, plan.profile.engine_id);
  assert.equal(manifest.profile, plan.profile.profile_id);
  assert.equal(manifest.speech_plan.plan_id, plan.plan_id);
  assert.equal(manifest.speech_plan.plan_digest, plan.plan_digest);
  const revision =
    plan.profile.engine_id === 'voxcpm2' ? 'openbmb/VoxCPM2' : 'ESpeech/ESpeech-TTS-1_RL-V2';
  assert.equal(manifest.runtime.model_revisions[revision], plan.profile.model_revision);
  assert.deepEqual(
    manifest.speech_plan.expected_utterance_ids,
    plan.utterances.map((u) => u.utterance_id),
  );
  assert.equal(
    new Set(manifest.segments.map((s) => s.chunk_id)).size,
    manifest.segments.length,
    'Repeated output chunk',
  );
  const cast = new Map(plan.cast.map((c) => [c.role_id, c.voice_id]));
  let index = 0;
  for (const utterance of plan.utterances) {
    let text = '',
      chunks = 0;
    while (manifest.segments[index]?.utterance_id === utterance.utterance_id) {
      const segment = manifest.segments[index++];
      chunks++;
      assert.equal(
        segment.chunk_id,
        `${utterance.utterance_id}-chunk-${String(chunks).padStart(4, '0')}`,
      );
      assert.equal(segment.cue_id, utterance.cue_id);
      assert.equal(segment.source_start_ms, utterance.start_ms);
      assert.equal(segment.source_end_ms, utterance.end_ms);
      assert.equal(segment.resolved_voice, cast.get(utterance.role_id));
      assert.equal(segment.reference_id, utterance.reference_id);
      assert.equal(segment.engine, plan.profile.engine_id);
      assert.ok(['generated', 'cached', 'reused_in_job'].includes(segment.status));
      assert.ok(segment.duration_ms > 0);
      text += segment.text;
    }
    assert.ok(chunks > 0, 'Missing utterance chunks');
    assert.equal(text, utterance.text, 'Output chunks lost or changed approved text');
  }
  assert.equal(index, manifest.segments.length, 'Extra or reordered output chunks');
  return { utterances: plan.utterances.length, chunks: index };
}
