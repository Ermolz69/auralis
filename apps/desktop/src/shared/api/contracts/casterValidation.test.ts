import { describe, expect, it } from 'vitest';
import { validateCastResult, validateVoiceCatalogPage } from './casterValidation';
import { validateArtifact } from './runtimeValidation';

const hash = 'a'.repeat(64);
const catalogFields = {
  description: null,
  manifest_sha256: null,
  reference_count: null,
  qualified_capabilities: null,
  error_code: null,
};

describe('Caster worker response boundaries', () => {
  it('accepts a ready managed cast artifact from the desktop index', () => {
    expect(
      validateArtifact({
        id: '00000000-0000-4000-8000-000000000001',
        kind: 'manualCast',
        location: { kind: 'storageKey', value: 'project/casting/revisions/a.json' },
        sizeBytes: null,
        state: 'ready',
        createdAt: '2026-10-10T00:00:00Z',
        updatedAt: '2026-10-10T00:00:00Z',
        readyAt: '2026-10-10T00:00:00Z',
      }),
    ).toBe(true);
  });

  it('accepts complete valid and legacy catalog entries and rejects missing fields', () => {
    const valid = {
      ...catalogFields,
      voice_id: 'voice-A',
      state: 'valid',
      display_name: 'Voice A',
      description: 'Prepared voice',
      manifest_sha256: hash,
      reference_count: 1,
      qualified_capabilities: [{ engine_id: 'voxcpm2', emotion_id: 'neutral' }],
    };
    const legacy = {
      ...catalogFields,
      voice_id: 'old-voice',
      state: 'legacy',
      display_name: 'Old voice',
    };
    expect(validateVoiceCatalogPage({ items: [valid, legacy], next_after: null })).toBe(true);
    const incomplete: Record<string, unknown> = { ...valid };
    Reflect.deleteProperty(incomplete, 'error_code');
    expect(validateVoiceCatalogPage({ items: [incomplete], next_after: null })).toBe(false);
  });

  it('requires the exact digest and role snapshot in a published revision', () => {
    const result = {
      revision_sha256: hash,
      revision: {
        version: 1,
        parent_revision: null,
        request_id: 'request-1',
        request_sha256: hash,
        cast: {
          version: 2,
          roles: { narrator: { display_name: 'Narrator', voice_id: 'voice-A' } },
        },
        selections: {
          narrator: {
            voice_id: 'voice-A',
            reference_id: 'ref-1',
            engine_id: 'voxcpm2',
            emotion_id: 'neutral',
            manifest_sha256: hash,
            audio_sha256: hash,
            report_sha256: hash,
            transcript: 'A line',
          },
        },
      },
    };
    expect(validateCastResult(result)).toBe(true);
    expect(validateCastResult({ ...result, revision_sha256: 'bad' })).toBe(false);
  });
});
