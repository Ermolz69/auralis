import assert from 'node:assert/strict';

export function verifyRawLayout(plan, sourcePlanHash, speech, layout, manifest, rawWav, chunks) {
  assert.equal(manifest.output_sha256, plan.raw_wav.sha256);
  assert.equal(layout.aggregate_sha256, plan.raw_wav.sha256);
  assert.equal(layout.source_plan_sha256, sourcePlanHash);
  assert.equal(layout.raw_manifest_sha256, plan.raw_manifest.sha256);
  assert.equal(plan.timeline.source_plan_sha256, sourcePlanHash);
  assert.equal(plan.timeline.raw_manifest_sha256, plan.raw_manifest.sha256);
  assert.equal(layout.assembly_policy, 'pcm16_stream_fade10ms_peak089125_kaiser5_v1');
  assert.equal(manifest.audio_assembly, layout.assembly_policy);
  assert.equal(manifest.sample_rate, 48000);
  assert.equal(layout.sample_rate, 48000);
  assert.equal(layout.channels, rawWav.channels);
  assert.equal(
    layout.frame_count,
    rawWav.frames,
    'Raw WAV does not contain complete layout frames',
  );
  assert.deepEqual(
    layout.utterances.map((u) => u.utterance_id),
    speech.utterances.map((u) => u.utterance_id),
  );
  assert.equal(chunks.length, manifest.segments.length);
  let offset = 0,
    index = 0;
  for (let i = 0; i < layout.utterances.length; i++) {
    const range = layout.utterances[i],
      utterance = speech.utterances[i],
      measured = plan.timeline.utterances[i];
    assert.equal(range.first_frame, offset);
    const ids = [];
    let frames = 0;
    while (manifest.segments[index]?.utterance_id === range.utterance_id) {
      const segment = manifest.segments[index],
        chunk = chunks[index++];
      assert.equal(chunk.channels, rawWav.channels);
      frames += Math.ceil((chunk.frames * 48000) / chunk.sample_rate);
      ids.push(segment.chunk_id);
    }
    assert.ok(frames > 0);
    assert.deepEqual(range.chunk_ids, ids, 'Layout chunk coverage changed');
    assert.equal(
      range.frame_count,
      frames,
      'Technical chunk frames do not cover the raw utterance',
    );
    const window =
      utterance.start_ms == null
        ? null
        : { start_ms: utterance.start_ms, end_ms: utterance.end_ms };
    assert.deepEqual(range.source_window, window);
    assert.deepEqual(measured.source_window, window);
    assert.equal(measured.frame_count, frames);
    assert.equal(measured.sample_rate, 48000);
    assert.equal(measured.channels, rawWav.channels);
    offset += frames;
  }
  assert.equal(offset, rawWav.frames, 'Raw WAV was shortened despite complete chunk metadata');
  assert.equal(index, manifest.segments.length);
}
