import assert from 'node:assert/strict';
import { test } from 'node:test';
import { verifyTranslationLineage } from './lineage-translation.mjs';
const cues = [{ id: 'cue-1', start_ms: 0, end_ms: 1000, text: 'Редакция.' }];
const review = {
  project_id: 'p',
  source: { kind: 'translation', source_id: 't', revision: 'r-3', sha256: 'output', cues },
};
const link = {
  project_id: 'p',
  translation_id: 't',
  selected_result_id: 'r',
  source_artifact_id: 'source',
  source_sha256: 'source-sha',
  source_format: 'srt',
  source_language: 'zh',
  target_language: 'ru',
  revision: 3,
  output_artifact_id: 'out',
};
const pub = {
  result_id: 'r',
  translation_id: 't',
  artifact_id: 'out',
  source_sha256: 'source-sha',
  state: 'ready',
};
const decision = {
  result_id: 'r',
  result_sha256: 'output',
  decision: 'approved',
  event_id: 'approved-event',
};
const verify = (r = review, l = link, p = pub, d = decision) =>
  verifyTranslationLineage(
    r,
    l,
    p,
    d,
    { source_subtitle_sha256: 'source-sha' },
    { translation_id: 't', result_id: 'r' },
    'source',
    'output',
    cues,
  );
test('full source rejects non-translation snapshots and mismatched approved result lineage', () => {
  assert.equal(verify().result_id, 'r');
  assert.throws(() => verify({ ...review, source: { ...review.source, kind: 'artifact_srt' } }));
  assert.throws(() => verify(review, { ...link, selected_result_id: 'other' }));
  assert.throws(() => verify(review, { ...link, source_artifact_id: 'other' }));
  assert.throws(() => verify(review, link, pub, { ...decision, decision: 'withdrawn' }));
  assert.throws(() =>
    verify({
      ...review,
      source: { ...review.source, cues: [{ ...cues[0], text: 'Another text' }] },
    }),
  );
});
