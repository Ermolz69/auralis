// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { SpeechCatalog } from './SpeechCatalog';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ invoke }));

const catalog = {
  profiles: [
    { profile_id: 'profile-one', profile_sha256: 'a'.repeat(64), engine_id: 'voxcpm2' },
    { profile_id: 'profile-two', profile_sha256: 'b'.repeat(64), engine_id: 'espeech' },
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
          engine_id: 'voxcpm2',
          emotion_id: 'neutral',
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
          engine_id: 'espeech',
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
  it('selects a qualified neutral reference automatically and refuses an unchecked engine voice', async () => {
    invoke.mockImplementation((command: string) =>
      Promise.resolve(command === 'get_speech_catalog_cmd' ? catalog : []),
    );
    render(<SpeechCatalog projectId="project-one" />);
    expect(await screen.findByRole('option', { name: 'Голос Один' })).not.toBeNull();
    expect(screen.queryByRole('option', { name: 'Голос Два' })).toBeNull();
    expect(invoke).toHaveBeenCalledWith('get_speech_catalog_cmd', { projectId: 'project-one' });

    expect(screen.getByLabelText('Голос')).toHaveProperty('value', 'voice-one');
    expect(screen.getByLabelText('Референс')).toHaveProperty('value', 'ref-one');
    expect(screen.getByText('Нейтральный образец подтверждён прослушиванием.')).not.toBeNull();

    fireEvent.change(screen.getByLabelText('Движок и профиль'), {
      target: { value: 'profile-two' },
    });
    expect(screen.queryByText('Нейтральный образец подтверждён прослушиванием.')).toBeNull();
    expect(screen.getByRole('option', { name: 'Голос Два' })).not.toBeNull();
    fireEvent.change(screen.getByLabelText('Голос'), { target: { value: 'voice-two' } });
    expect(screen.getByLabelText('Референс')).toHaveProperty('value', '');
    expect(
      screen.getByText('Для этого голоса ещё нужен подтверждённый нейтральный образец.'),
    ).not.toBeNull();
  });

  it('shows a catalog loading failure without exposing an empty selection', async () => {
    invoke.mockRejectedValue(new Error('Catalog unavailable'));
    render(<SpeechCatalog projectId="project-two" />);
    expect(await screen.findByRole('alert')).toHaveProperty(
      'textContent',
      expect.stringContaining('Catalog unavailable'),
    );
    expect(screen.queryByLabelText('Голос')).toBeNull();
  });
});
