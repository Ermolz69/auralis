import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type {
  CastResult,
  VoiceDetails,
  VoiceReferenceMetadata,
} from '@/shared/api/contracts/caster';
import { useVoiceCatalog } from './useVoiceCatalog';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ invoke }));
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (path: string) => 'asset:' + path }));
const reference: VoiceReferenceMetadata = {
  reference_id: 'reference',
  engine_id: 'engine',
  emotion_id: 'neutral',
  transcript: 'Fixture',
  audio_sha256: 'a'.repeat(64),
  qualification_status: 'qualified',
  qualification_report_sha256: 'b'.repeat(64),
};
const details: VoiceDetails = {
  voice_id: 'voice',
  display_name: 'Fixture voice',
  description: 'Prepared fixture',
  manifest_sha256: 'c'.repeat(64),
  references: [reference],
  next_after_reference: 'reference',
};
const saved: CastResult = {
  revision_sha256: 'd'.repeat(64),
  revision: {
    version: 1,
    parent_revision: null,
    request_id: 'request',
    request_sha256: 'e'.repeat(64),
    cast: { version: 2, roles: { narrator: { display_name: 'Narrator', voice_id: 'voice' } } },
    selections: {
      narrator: {
        voice_id: 'voice',
        reference_id: 'reference',
        engine_id: 'engine',
        emotion_id: 'neutral',
        manifest_sha256: 'c'.repeat(64),
        audio_sha256: 'a'.repeat(64),
        report_sha256: 'b'.repeat(64),
        transcript: 'Fixture',
      },
    },
  },
};
beforeEach(() => {
  invoke.mockReset().mockImplementation(async (command) => {
    if (command === 'list_voice_catalog_cmd')
      return { items: [{ voice_id: 'voice', state: 'valid' }], next_after: 'voice' };
    if (command === 'get_manual_cast_cmd') return null;
    if (command === 'inspect_voice_cmd') return details;
    if (command === 'inspect_voice_reference_cmd')
      return { ...reference, voice_id: 'voice', playback_path: 'fixture.wav' };
    if (command === 'check_voice_compatibility_cmd')
      return { compatible: true, reference_ids: ['reference'] };
    if (command === 'assign_manual_cast_cmd') return saved;
    if (command === 'recover_manual_cast_cmd') return null;
    throw new Error('Unexpected command: ' + command);
  });
});
afterEach(cleanup);
async function ready() {
  const hook = renderHook(({ projectId }) => useVoiceCatalog(projectId), {
    initialProps: { projectId: 'project' },
  });
  await waitFor(() => expect(hook.result.current.voices).toHaveLength(1));
  return hook;
}
async function selected() {
  const hook = await ready();
  await act(async () => {
    await hook.result.current.inspectVoice('voice');
  });
  await act(async () => {
    await hook.result.current.chooseReference(reference);
  });
  return hook;
}

it('hydrates revision roles and strips frozen receipt metadata from editable selections', async () => {
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) =>
    command === 'get_manual_cast_cmd' ? saved : original(command, args),
  );
  const { result } = await ready();
  expect(result.current.cast).toBe(saved);
  expect(result.current.roles).toEqual(saved.revision.cast.roles);
  expect(result.current.selections.narrator).toEqual({
    voice_id: 'voice',
    reference_id: 'reference',
    engine_id: 'engine',
    emotion_id: 'neutral',
  });
});

it('paginates voices, bounds repeated loading and retains the accepted page on failure', async () => {
  const { result } = await ready();
  invoke.mockResolvedValueOnce({ items: [{ voice_id: 'next' }], next_after: null });
  await act(async () => {
    await result.current.loadMoreVoices();
  });
  expect(result.current.voices.map((v) => v.voice_id)).toEqual(['voice', 'next']);
  expect(invoke).toHaveBeenLastCalledWith('list_voice_catalog_cmd', {
    projectId: 'project',
    after: 'voice',
    limit: 20,
  });
  const count = invoke.mock.calls.length;
  await act(async () => {
    await result.current.loadMoreVoices();
  });
  expect(invoke).toHaveBeenCalledTimes(count);
});

it('reports pagination failure and permits retry from the same cursor', async () => {
  const { result } = await ready();
  invoke.mockRejectedValueOnce(new Error('Page unavailable'));
  await act(async () => {
    await result.current.loadMoreVoices();
  });
  expect(result.current.error).toBe('Page unavailable');
  expect(result.current.voices).toHaveLength(1);
  expect(result.current.nextVoice).toBe('voice');
  await act(async () => {
    await result.current.loadMoreVoices();
  });
  expect(result.current.busy).toBe(false);
});

it('appends references only to the same voice, resets playback for a new voice and reports inspection errors', async () => {
  const { result } = await ready();
  await act(async () => {
    await result.current.inspectAudio(reference);
  });
  expect(
    invoke.mock.calls.filter(([command]) => command === 'inspect_voice_reference_cmd'),
  ).toHaveLength(0);
  await act(async () => {
    await result.current.inspectVoice('voice');
  });
  await act(async () => {
    await result.current.inspectAudio(reference);
  });
  expect(result.current.playback).toBe('asset:fixture.wav');
  invoke.mockResolvedValueOnce({
    ...details,
    references: [{ ...reference, reference_id: 'next' }],
    next_after_reference: null,
  });
  await act(async () => {
    await result.current.inspectVoice('voice', 'reference');
  });
  expect(result.current.voice?.references.map((r) => r.reference_id)).toEqual([
    'reference',
    'next',
  ]);
  expect(result.current.playback).toBe('asset:fixture.wav');
  await act(async () => {
    await result.current.inspectVoice('voice');
  });
  expect(result.current.playback).toBeNull();
  invoke.mockRejectedValueOnce(new Error('Runtime unavailable'));
  await act(async () => {
    await result.current.inspectVoice('another');
  });
  expect(result.current.error).toBe('Runtime unavailable');
  expect(result.current.voice?.voice_id).toBe('voice');
});

it.each(['unverified', 'rejected'] as const)(
  'does not admit a %s reference',
  async (qualification_status) => {
    const { result } = await ready();
    await act(async () => {
      await result.current.chooseReference(reference);
    });
    await act(async () => {
      await result.current.inspectVoice('voice');
    });
    const count = invoke.mock.calls.length;
    await act(async () => {
      await result.current.chooseReference({ ...reference, qualification_status });
    });
    expect(invoke).toHaveBeenCalledTimes(count);
    expect(result.current.selections).toEqual({});
  },
);

it.each([
  ['invalid id', 'Narrator'],
  ['valid_id', '   '],
])('validates role %s before compatibility admission', async (id, name) => {
  const { result } = await ready();
  await act(async () => {
    await result.current.inspectVoice('voice');
  });
  act(() => {
    result.current.setRoleId(id);
    result.current.setRoleName(name);
  });
  const count = invoke.mock.calls.length;
  await act(async () => {
    await result.current.chooseReference(reference);
  });
  expect(result.current.error).toContain('Укажите ID роли');
  expect(invoke).toHaveBeenCalledTimes(count);
});

it.each([
  { compatible: false, reference_ids: ['reference'] },
  { compatible: true, reference_ids: ['different'] },
])('rejects compatibility without the exact qualified reference (%j)', async (compatible) => {
  const { result } = await ready();
  await act(async () => {
    await result.current.inspectVoice('voice');
  });
  invoke.mockResolvedValueOnce(compatible);
  await act(async () => {
    await result.current.chooseReference(reference);
  });
  expect(result.current.selections).toEqual({});
  expect(result.current.error).toContain('Голос не подтверждён');
  expect(result.current.busy).toBe(false);
});

it('keeps the accepted selection and request identity after uncertain save, then recovers and retries', async () => {
  const { result } = await selected();
  const accepted = result.current.selections;
  invoke.mockRejectedValueOnce({
    code: 'RECOVERY_REQUIRED',
    message: 'Recover the current cast, then retry.',
  });
  await act(async () => {
    await result.current.saveCast();
  });
  expect(result.current.error).toBe('Recover the current cast, then retry.');
  expect(result.current.selections).toBe(accepted);
  const first = invoke.mock.calls.find(([command]) => command === 'assign_manual_cast_cmd')![1];
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) =>
    command === 'get_manual_cast_cmd' ? saved : original(command, args),
  );
  await act(async () => {
    await result.current.recover();
  });
  expect(result.current.notice).toBe('Ревизия восстановлена.');
  expect(result.current.selections).toBe(accepted);
  await act(async () => {
    await result.current.saveCast();
  });
  const calls = invoke.mock.calls.filter(([command]) => command === 'assign_manual_cast_cmd');
  expect(calls[1][1]).toMatchObject({
    expectedRevision: saved.revision_sha256,
    requestId: first.requestId,
  });
  expect(result.current.notice).toContain('сохранена');
});

it('reports playback and recovery errors without inventing a new selection or revision', async () => {
  const { result } = await ready();
  await act(async () => {
    await result.current.saveCast();
  });
  expect(
    invoke.mock.calls.filter(([command]) => command === 'assign_manual_cast_cmd'),
  ).toHaveLength(0);
  await act(async () => {
    await result.current.inspectVoice('voice');
  });
  invoke.mockRejectedValueOnce('Playback unavailable');
  await act(async () => {
    await result.current.inspectAudio(reference);
  });
  expect(result.current.error).toBe('Playback unavailable');
  expect(result.current.playback).toBeNull();
  invoke.mockRejectedValueOnce({});
  await act(async () => {
    await result.current.recover();
  });
  expect(result.current.error).toBe('Операция с голосами не выполнена');
  expect(result.current.cast).toBeNull();
  expect(result.current.selections).toEqual({});
});

it('ignores stale initial catalog responses after project switching and unmount', async () => {
  let resolve!: (page: unknown) => void;
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => {
    if (command === 'list_voice_catalog_cmd' && args.projectId === 'old')
      return new Promise((yes) => {
        resolve = yes;
      });
    return original(command, args);
  });
  const hook = renderHook(({ projectId }) => useVoiceCatalog(projectId), {
    initialProps: { projectId: 'old' },
  });
  hook.rerender({ projectId: 'new' });
  await waitFor(() => expect(hook.result.current.voices).toHaveLength(1));
  await act(async () => {
    resolve({ items: [{ voice_id: 'stale' }], next_after: null });
  });
  expect(hook.result.current.voices[0].voice_id).toBe('voice');
  hook.unmount();
});

it('reports initial catalog errors while a mounted project is active', async () => {
  invoke.mockRejectedValue(new Error('Catalog unavailable'));
  const { result } = renderHook(() => useVoiceCatalog('project'));
  await waitFor(() => expect(result.current.error).toBe('Catalog unavailable'));
  expect(result.current.voices).toEqual([]);
});
