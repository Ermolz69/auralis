import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createHash } from 'node:crypto';

export function verifyEvidence(record, read = fs.readFileSync) {
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
        assert.match(report.case_id ?? '', /^[A-Za-z0-9][A-Za-z0-9._-]{0,120}$/);
        assert.deepEqual(report.engine_ids, ['voxcpm2', 'espeech']);
      }
    }
  }
}
