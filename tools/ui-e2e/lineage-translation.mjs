import assert from 'node:assert/strict';

export function verifyTranslationLineage(
  review,
  link,
  publication,
  decision,
  spec,
  ui,
  sourceId,
  outputSha,
  cues,
) {
  assert.equal(
    review.source.kind,
    'translation',
    'Full source speech must use the approved translation',
  );
  assert.equal(review.source.source_id, ui.translation_id);
  assert.equal(link.translation_id, ui.translation_id);
  assert.equal(link.project_id, review.project_id);
  assert.equal(link.selected_result_id, ui.result_id);
  assert.equal(link.source_artifact_id, sourceId);
  assert.equal(link.source_sha256, spec.source_subtitle_sha256);
  assert.equal(link.source_format, 'srt');
  assert.equal(link.source_language, 'zh');
  assert.equal(link.target_language, 'ru');
  assert.equal(review.source.revision, `${ui.result_id}-${link.revision}`);
  assert.equal(publication.result_id, ui.result_id);
  assert.equal(publication.translation_id, ui.translation_id);
  assert.equal(publication.artifact_id, link.output_artifact_id);
  assert.equal(publication.source_sha256, link.source_sha256);
  assert.equal(publication.state, 'ready');
  assert.equal(decision.result_id, ui.result_id);
  assert.equal(decision.result_sha256, review.source.sha256);
  assert.equal(decision.decision, 'approved');
  assert.equal(outputSha, review.source.sha256);
  assert.deepEqual(
    review.source.cues.map((c) => [c.id, c.start_ms, c.end_ms, c.text]),
    cues.map((c) => [c.id, c.start_ms, c.end_ms, c.text]),
    'Approved snapshot differs from the selected published translation bytes',
  );
  return {
    translation_id: ui.translation_id,
    result_id: ui.result_id,
    result_sha256: outputSha,
    link_revision: link.revision,
    event_id: decision.event_id,
  };
}
