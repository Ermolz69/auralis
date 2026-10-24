import { describe, expect, it } from 'vitest';
import { validateAssemblyPreview, validateAssemblyView } from './audioAssemblyValidation';

const hash = 'a'.repeat(64);
const settings = { mode: 'continuous', source_duration_ms: null, pause_after_ms: [0] };
const measured = {
  utterance_id: 'one',
  audio_sha256: hash,
  sample_rate: 48000,
  channels: 1,
  frame_count: 4800,
  source_window: { start_ms: 100, end_ms: 500 },
  pause_after_ms: 0,
};
const preview = {
  source_run_id: 'raw',
  source_plan_sha256: hash,
  preview_sha256: hash,
  profile_id: 'fixture',
  settings,
  sample_rate: 48000,
  channels: 1,
  frames: 4800,
  utterances: [measured],
  conflicts: [],
  renderer_available: true,
  renderer_error: null,
};
const job = {
  id: 'job',
  kind: 'audioAssembly',
  revision: 2,
  projectId: 'project',
  title: 'Assembly',
  status: 'completed',
  stage: 'exportResult',
  progress: {
    percent: 100,
    message: 'Ready',
    currentStep: null,
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: '2026-10-23T00:00:00Z',
  updatedAt: '2026-10-23T00:00:01Z',
};
const files = ['wav', 'manifest'].map((kind) => ({
  artifact_id: kind,
  kind,
  sha256: hash,
  size_bytes: 123,
}));
const publication = {
  assembly_id: 'assembly',
  attempt_id: 'attempt',
  sample_rate: 48000,
  channels: 1,
  frames: 4800,
  pcm_sha256: hash,
  peak_sample_abs: 100,
  full_scale_samples: 0,
  ready: true,
  files,
};
const view = {
  assembly_id: 'assembly',
  source_run_id: 'raw',
  plan_sha256: hash,
  settings,
  utterance_count: 1,
  frames: 4800,
  job,
  result: publication,
};

describe('managed audio assembly IPC', () => {
  it('accepts a complete immutable preview and 600 distinct utterances', () => {
    expect(validateAssemblyPreview(preview)).toBe(true);
    const utterances = Array.from({ length: 600 }, (_, index) => ({
      ...measured,
      utterance_id: `u-${index}`,
    }));
    expect(
      validateAssemblyPreview({
        ...preview,
        utterances,
        settings: { ...settings, pause_after_ms: Array(600).fill(0) },
      }),
    ).toBe(true);
    expect(
      validateAssemblyPreview({
        ...preview,
        utterances: [measured, measured],
        settings: { ...settings, pause_after_ms: [0, 0] },
      }),
    ).toBe(false);
    expect(
      validateAssemblyPreview({ ...preview, settings: { ...settings, pause_after_ms: [] } }),
    ).toBe(false);
  });
  it('requires typed actionable conflicts and refuses contradictory completion', () => {
    const conflict = {
      kind: 'impossible_fit',
      utterance_id: 'one',
      required_frames: 4800,
      available_frames: 48,
    };
    expect(validateAssemblyPreview({ ...preview, frames: null, conflicts: [conflict] })).toBe(true);
    expect(validateAssemblyPreview({ ...preview, conflicts: [conflict] })).toBe(false);
    expect(
      validateAssemblyPreview({
        ...preview,
        frames: null,
        conflicts: [{ kind: 'impossible_fit', utterance_id: 'one' }],
      }),
    ).toBe(false);
    expect(
      validateAssemblyPreview({ ...preview, frames: null, conflicts: [{ kind: 'unknown' }] }),
    ).toBe(false);
    expect(
      validateAssemblyPreview({
        ...preview,
        frames: null,
        conflicts: [{ kind: 'invalid_window', utterance_id: 'one', start_ms: 100, end_ms: 50 }],
        utterances: [{ ...measured, source_window: { start_ms: 100, end_ms: 50 } }],
      }),
    ).toBe(true);
  });
  it('requires a ready matching group with separate WAV and manifest', () => {
    expect(validateAssemblyView(view)).toBe(true);
  });
  it.each([
    { ...publication, ready: false },
    { ...publication, assembly_id: 'other' },
    { ...publication, frames: 4801 },
    { ...publication, files: [files[0]] },
    { ...publication, files: [files[0], files[0]] },
    { ...publication, peak_sample_abs: 32769 },
    { ...publication, full_scale_samples: 4801 },
  ])('refuses a partial, changed or impossible publication %#', (result) => {
    expect(validateAssemblyView({ ...view, result })).toBe(false);
  });
  it('keeps pending history metadata valid without exposing a result', () => {
    expect(validateAssemblyView({ ...view, job: null, result: null })).toBe(true);
    expect(validateAssemblyView({ ...view, job: { ...job, kind: 'speech' } })).toBe(false);
    expect(
      validateAssemblyView({
        ...view,
        job: { ...job, status: 'running', progress: { ...job.progress, percent: 90 } },
      }),
    ).toBe(false);
    expect(validateAssemblyView({ ...view, frames: Number.MAX_SAFE_INTEGER + 1 })).toBe(false);
  });
});
