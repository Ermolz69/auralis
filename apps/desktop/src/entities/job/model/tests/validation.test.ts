import { describe, expect, it } from 'vitest';
import { validateJobDto, validateJobEventDto, validateJobSnapshot } from '../validation';
import type { JobDto } from '../types';

const job: JobDto = {
  id: 'job-1',
  kind: 'dubbing',
  projectId: 'p1',
  revision: 1,
  title: 'Job',
  status: 'pending',
  stage: null,
  error: null,
  createdAt: '',
  updatedAt: '',
  progress: { percent: 0, message: '', currentStep: null, processedItems: null, totalItems: null },
};

describe('job kind validation', () => {
  it.each([undefined, null, 42, '', '   '])('rejects missing or invalid kind %s', (kind) => {
    const invalid = { ...job, kind };
    expect(validateJobDto(invalid)).toBe(false);
    expect(validateJobSnapshot([invalid])).toBe(false);
    expect(validateJobEventDto({ kind: 'created', job: invalid })).toBe(false);
  });

  it('keeps future job kinds valid without confusing them with lifecycle event kinds', () => {
    const future = { ...job, kind: 'export' };
    expect(validateJobSnapshot([future])).toBe(true);
    expect(validateJobEventDto({ kind: 'created', job: future })).toBe(true);
    expect(validateJobEventDto({ kind: 'dubbing', job })).toBe(false);
  });
});
