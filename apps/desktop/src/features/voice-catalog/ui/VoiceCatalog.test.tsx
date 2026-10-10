// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { VoiceCatalog } from './VoiceCatalog';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ invoke }));
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (path: string) => `asset:${path}` }));

const projectId = '00000000-0000-4000-8000-000000000001';
const digest = 'a'.repeat(64);
const reference = {
  reference_id: 'reference-1',
  engine_id: 'espeech',
  emotion_id: 'neutral',
  transcript: 'Recorded words',
  audio_sha256: digest,
  qualification_status: 'qualified',
  qualification_report_sha256: 'b'.repeat(64),
};
const saved = {
  revision_sha256: 'c'.repeat(64),
  revision: {
    version: 1,
    parent_revision: null,
    request_id: 'request-1',
    request_sha256: 'd'.repeat(64),
    cast: { version: 2, roles: { narrator: { display_name: 'Рассказчик', voice_id: 'voice-1' } } },
    selections: {
      narrator: {
        voice_id: 'voice-1',
        reference_id: 'reference-1',
        engine_id: 'espeech',
        emotion_id: 'neutral',
        manifest_sha256: digest,
        audio_sha256: digest,
        report_sha256: 'b'.repeat(64),
        transcript: 'Recorded words',
      },
    },
  },
};

beforeEach(() => {
  vi.stubGlobal('crypto', { randomUUID: () => 'request-1' });
  invoke.mockImplementation(async (command: string) => {
    if (command === 'list_voice_catalog_cmd')
      return {
        items: [
          {
            voice_id: 'voice-1',
            state: 'valid',
            display_name: 'Fixture voice',
            description: 'Prepared',
            manifest_sha256: digest,
            reference_count: 1,
            qualified_capabilities: [{ engine_id: 'espeech', emotion_id: 'neutral' }],
            error_code: null,
          },
        ],
        next_after: null,
      };
    if (command === 'get_manual_cast_cmd') return null;
    if (command === 'inspect_voice_cmd')
      return {
        voice_id: 'voice-1',
        display_name: 'Fixture voice',
        description: 'Prepared',
        manifest_sha256: digest,
        references: [reference],
        next_after_reference: null,
      };
    if (command === 'inspect_voice_reference_cmd')
      return { ...reference, voice_id: 'voice-1', playback_path: 'C:/cache/reference.wav' };
    if (command === 'check_voice_compatibility_cmd')
      return {
        voice_id: 'voice-1',
        engine_id: 'espeech',
        emotion_id: 'neutral',
        compatible: true,
        reference_ids: ['reference-1'],
      };
    if (command === 'assign_manual_cast_cmd') return saved;
    throw new Error(`Unexpected command ${command}`);
  });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});

it('inspects and plays a prepared reference, then saves an explicit cast through typed IPC', async () => {
  render(<VoiceCatalog projectId={projectId} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Открыть' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Слушать' }));
  await waitFor(() =>
    expect(document.querySelector('audio')?.getAttribute('src')).toBe(
      'asset:C:/cache/reference.wav',
    ),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Назначить роли' }));
  await screen.findByText(/Для роли narrator выбран/);
  fireEvent.click(screen.getByRole('button', { name: 'Сохранить cast' }));
  await screen.findByText(/Ревизия cccccccccccc сохранена/);
  expect(invoke).toHaveBeenCalledWith('assign_manual_cast_cmd', {
    projectId,
    cast: { version: 2, roles: { narrator: { display_name: 'Рассказчик', voice_id: 'voice-1' } } },
    selections: {
      narrator: {
        voice_id: 'voice-1',
        reference_id: 'reference-1',
        engine_id: 'espeech',
        emotion_id: 'neutral',
      },
    },
    expectedRevision: null,
    requestId: 'request-1',
  });
});

it('retains the same request identity when a save response is uncertain', async () => {
  let attempts = 0;
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command: string, args: unknown) => {
    if (command === 'assign_manual_cast_cmd' && attempts++ === 0)
      throw new Error('Transport response lost');
    return original(command, args);
  });
  render(<VoiceCatalog projectId={projectId} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Открыть' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Назначить роли' }));
  await screen.findByText(/Для роли narrator выбран/);
  fireEvent.click(screen.getByRole('button', { name: 'Сохранить cast' }));
  await screen.findByText('Transport response lost');
  fireEvent.click(screen.getByRole('button', { name: 'Сохранить cast' }));
  await screen.findByText(/Ревизия cccccccccccc сохранена/);
  const requests = invoke.mock.calls.filter(([command]) => command === 'assign_manual_cast_cmd');
  expect(requests).toHaveLength(2);
  expect(requests[0][1].requestId).toBe(requests[1][1].requestId);
});
