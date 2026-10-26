import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createHash } from 'node:crypto';

export function verifyEvidence(record, read = fs.readFileSync) {
  assert.deepEqual(
    record.frozen_cases?.map((item) => item.case_scope),
    ['diagnostic', 'full-source'],
    'Missing frozen diagnostic/full-source case identities',
  );
  for (const item of record.frozen_cases) {
    assert.match(item.case_id ?? '', /^[A-Za-z0-9][A-Za-z0-9._-]{0,120}$/);
    assert.match(item.spec_sha256 ?? '', /^[a-f0-9]{64}$/);
  }
  assert.equal(
    new Set(record.frozen_cases.map((item) => item.case_id)).size,
    2,
    'Diagnostic and full-source require distinct frozen cases',
  );
  assert.equal(
    new Set(record.frozen_cases.map((item) => item.spec_sha256)).size,
    2,
    'Diagnostic and full-source require distinct frozen specification digests',
  );
  for (const task of record.tasks) {
    assert.ok(task.evidence?.length, `Missing evidence for ${task.id}`);
    for (const evidence of task.evidence) {
      assert.match(evidence.path ?? '', /^tools\/revoice\/evidence\/[A-Za-z0-9_-]+\.json$/);
      const bytes = read(evidence.path);
      assert.equal(createHash('sha256').update(bytes).digest('hex'), evidence.sha256);
      const report = JSON.parse(bytes);
      assert.equal(report.version, 1);
      assert.equal(report.task_id, task.id);
      assert.equal(report.status, 'passed', 'Evidence did not pass');
      assert.equal(report.candidate_sha256, record.candidate_executable_sha256);
      assert.equal(report.source_subtitle_sha256, record.source.subtitle_sha256);
      assert.equal(report.runtime_seed_sha256, record.prospective_runtime.seed_manifest_sha256);
      assert.equal(report.source_cue_count, record.source.expected_cues);
      if (['RC-06', 'RC-07', 'RC-08'].includes(task.id)) {
        assert.equal(report.case_scope, task.id === 'RC-06' ? 'diagnostic' : 'full-source');
        const frozen = record.frozen_cases.find((item) => item.case_scope === report.case_scope);
        assert.equal(report.case_id, frozen.case_id, 'Evidence belongs to another frozen case');
        assert.equal(report.spec_sha256, frozen.spec_sha256, 'Frozen specification digest differs');
        assert.deepEqual(report.engine_ids, ['voxcpm2', 'espeech']);
      }
    }
  }
}
