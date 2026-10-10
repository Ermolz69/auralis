// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { SpeechCatalog } from './SpeechCatalog';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ invoke }));

const catalog = {
  profiles: [
    { profile_id: 'profile-one', profile_sha256: 'a'.repeat(64), engine_id: 'engine-one' },
    { profile_id: 'profile-two', profile_sha256: 'b'.repeat(64), engine_id: 'engine-two' },
  ],
  voices: [
    {
      voice_id: 'voice-one',
      display_name: 'Голос Один',
      description: 'First voice',
      manifest_sha256: 'c'.repeat(64),
      references: [
        {
          reference_id: 'ref-one',
          engine_id: 'engine-one',
          emotion_id: 'calm',
          transcript: 'Здравствуйте',
          audio_sha256: 'd'.repeat(64),
          qualification_status: 'qualified',
        },
      ],
    },
    {
      voice_id: 'voice-two',
      display_name: 'Голос Два',
      description: 'Second voice',
      manifest_sha256: 'e'.repeat(64),
      references: [
        {
          reference_id: 'ref-two',
          engine_id: 'engine-two',
          emotion_id: 'neutral',
          transcript: 'Привет',
          audio_sha256: 'f'.repeat(64),
          qualification_status: 'unverified',
        },
      ],
    },
  ],
};

beforeEach(() => vi.clearAllMocks());
afterEach(() => cleanup());

describe('SpeechCatalog', () => {
  it('filters references by engine and resets voice selection when the profile changes', async () => {
    invoke.mockResolvedValue(catalog);
    render(<SpeechCatalog projectId="project-one" />);
    expect(await screen.findByRole('option', { name: 'Голос Один' })).not.toBeNull();
    expect(screen.queryByRole('option', { name: 'Голос Два' })).toBeNull();
    expect(invoke).toHaveBeenCalledWith('get_speech_catalog_cmd', { projectId: 'project-one' });

    fireEvent.change(screen.getByLabelText('Голос'), { target: { value: 'voice-one' } });
    fireEvent.change(screen.getByLabelText('Референс'), { target: { value: 'ref-one' } });
    expect(screen.getByText('Референс подтверждён для выбранного движка.')).not.toBeNull();

    fireEvent.change(screen.getByLabelText('Движок и профиль'), {
      target: { value: 'profile-two' },
    });
    expect(screen.queryByText('Референс подтверждён для выбранного движка.')).toBeNull();
    expect(screen.getByRole('option', { name: 'Голос Два' })).not.toBeNull();
    fireEvent.change(screen.getByLabelText('Голос'), { target: { value: 'voice-two' } });
    fireEvent.change(screen.getByLabelText('Референс'), { target: { value: 'ref-two' } });
    expect(
      screen.getByText('Голос выбран; проверка референса для синтеза ещё не завершена.'),
    ).not.toBeNull();
  });

  it('shows a catalog loading failure without exposing an empty selection', async () => {
    invoke.mockRejectedValue(new Error('Catalog unavailable'));
    render(<SpeechCatalog projectId="project-two" />);
    expect(await screen.findByRole('status')).toHaveProperty(
      'textContent',
      'TTS недоступен: Catalog unavailable',
    );
    expect(screen.queryByLabelText('Голос')).toBeNull();
  });
});
