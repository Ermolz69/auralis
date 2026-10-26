import assert from 'node:assert/strict';

export function verifySpeechEvidence(speech, review, selections) {
  assert.equal(speech.review.status, 'approved');
  assert.equal(speech.review.decision_id, review.decision_id);
  assert.equal(speech.project_id, review.project_id);
  assert.equal(speech.source.source_sha256, review.source.sha256);
  assert.equal(speech.source.text_revision, review.source.revision);
  const cast = new Map(speech.cast.map((item) => [item.role_id, item.voice_id]));
  assert.equal(cast.size, speech.cast.length, 'Repeated cast role');
  assert.ok(speech.utterances.length > 0, 'Missing speech utterances');
  for (const utterance of speech.utterances)
    assert.ok(
      selections.some(
        (item) =>
          item.engine_id === speech.profile.engine_id &&
          item.profile_id === speech.profile.profile_id &&
          (!item.profile_sha256 || item.profile_sha256 === speech.profile.profile_sha256) &&
          (!item.model_revision || item.model_revision === speech.profile.model_revision) &&
          item.voice_id === cast.get(utterance.role_id) &&
          (!item.voice_manifest_sha256 ||
            speech.cast.find((c) => c.role_id === utterance.role_id)?.manifest_sha256 ===
              item.voice_manifest_sha256) &&
          item.reference_id === utterance.reference_id,
      ),
      'Speech selection differs from the frozen case',
    );
  if (review.source.kind === 'text') {
    assert.equal(speech.utterances.length, 1);
    assert.equal(speech.utterances[0].text, review.source.text);
  } else {
    assert.deepEqual(
      speech.utterances.map((item) => [item.cue_id, item.text, item.start_ms, item.end_ms]),
      review.source.cues.map((cue) => [cue.id, cue.text, cue.start_ms, cue.end_ms]),
      'Speech order, text or timing differs from the approved source',
    );
  }
}
