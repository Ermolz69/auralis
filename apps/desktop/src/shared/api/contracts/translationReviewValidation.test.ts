import { describe, expect, it } from 'vitest';
import { parseCommandResult } from './runtimeValidation';

const view = {
  resultId: '00000000-0000-4000-8000-000000000001',
  resultSha256: 'a'.repeat(64),
  linkRevision: 3,
  needsReview: true,
  selected: true,
  latest: {
    eventId: '00000000-0000-4000-8000-000000000002',
    decision: 'approved',
    reviewerLabel: 'Мария',
    note: 'Сверено с оригиналом',
    createdAt: '2026-10-10T00:00:00Z',
  },
};

describe('translation review IPC', () => {
  it('accepts the exact reviewed result identity', () => {
    expect(parseCommandResult('get_translation_review_cmd', view)).toEqual(view);
    expect(parseCommandResult('record_translation_review_cmd', view)).toEqual(view);
  });

  it('rejects a malformed digest or decision before the UI uses it', () => {
    expect(() =>
      parseCommandResult('get_translation_review_cmd', {
        ...view,
        resultSha256: 'not-a-digest',
      }),
    ).toThrow();
    expect(() =>
      parseCommandResult('record_translation_review_cmd', {
        ...view,
        latest: { ...view.latest, decision: 'automatically_approved' },
      }),
    ).toThrow();
  });
});
