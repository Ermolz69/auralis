import { expect, it } from 'vitest';
import { validateSpeechSource, validateSpeechRun } from './speechValidation';

const source = {
  kind: 'text',
  source_id: null,
  revision: 'a'.repeat(64),
  sha256: 'a'.repeat(64),
  text: 'Привет 🦊 [voice: literal]',
  cues: [],
};
const plan = {
  run_id: 'run',
  profile_id: 'profile',
  utterance_count: 1,
  chunk_count: 2,
  job: null,
  result: null,
};
it('rejects invalid source hashes and reversed cue windows without altering literal text', () => {
  expect(validateSpeechSource(source)).toBe(true);
  expect(validateSpeechSource({ ...source, sha256: 'changed' })).toBe(false);
  expect(
    validateSpeechSource({
      ...source,
      cues: [{ id: 'cue', role: 'role', text: 'реплика', start_ms: 2000, end_ms: 1000 }],
    }),
  ).toBe(false);
});
it('rejects unfinished publications, incomplete coverage and duplicate artifacts', () => {
  expect(validateSpeechRun(plan)).toBe(true);
  const job = {
    id: 'job',
    kind: 'speech',
    revision: 3,
    projectId: 'project',
    title: 'Синтез',
    status: 'completed',
    stage: null,
    progress: { percent: 100, message: '', currentStep: null, processedItems: 2, totalItems: 2 },
    error: null,
    createdAt: '2026-10-11T12:00:00Z',
    updatedAt: '2026-10-11T12:00:00Z',
  };
  const files = ['wav', 'manifest', 'segment', 'segment'].map((kind, q) => ({
    artifact_id: `file-${q}`,
    kind,
    sha256: 'b'.repeat(64),
    size_bytes: 100,
  }));
  const result = { run_id: 'run', attempt_id: 'attempt', duration_ms: 200, ready: true, files };
  expect(validateSpeechRun({ ...plan, job, result })).toBe(true);
  expect(validateSpeechRun({ ...plan, job, result: { ...result, ready: false } })).toBe(false);
  expect(validateSpeechRun({ ...plan, job, result: { ...result, files: files.slice(0, 3) } })).toBe(
    false,
  );
  expect(
    validateSpeechRun({
      ...plan,
      job,
      result: { ...result, files: [...files.slice(0, 3), files[2]] },
    }),
  ).toBe(false);
});
