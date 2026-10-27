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
        {
          reference_id: 'ref-joy',
          engine_id: 'voxcpm2',
          emotion_id: 'joy',
          transcript: 'Ура',
          audio_sha256: 'e'.repeat(64),
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
    expect(screen.getByText('Образец «neutral» подтверждён прослушиванием.')).not.toBeNull();
    fireEvent.change(screen.getByLabelText('Референс'), { target: { value: 'ref-joy' } });
    expect(screen.getByText('Образец «joy» подтверждён прослушиванием.')).not.toBeNull();

    fireEvent.change(screen.getByLabelText('Движок и профиль'), {
      target: { value: 'profile-two' },
    });
    expect(screen.queryByText('Образец «joy» подтверждён прослушиванием.')).toBeNull();
    expect(screen.getByRole('option', { name: 'Голос Два' })).not.toBeNull();
    fireEvent.change(screen.getByLabelText('Голос'), { target: { value: 'voice-two' } });
    expect(screen.getByLabelText('Референс')).toHaveProperty('value', '');
    expect(
      screen.getByText('Для этого голоса и движка ещё нужен подтверждённый образец.'),
    ).not.toBeNull();
  });

  it('requires explicit consent for an unverified neutral reference and revokes the selection when unchecked', async () => {
    render(<SpeechCatalog projectId="project-one" />);
    await screen.findByRole('option', { name: 'Голос Один' });
    fireEvent.change(screen.getByLabelText('Движок и профиль'), {
      target: { value: 'profile-two' },
    });
    fireEvent.change(screen.getByLabelText('Голос'), { target: { value: 'voice-two' } });
    expect(screen.getByLabelText('Референс')).toHaveProperty('value', '');
    fireEvent.click(screen.getByRole('checkbox', { name: /Разрешаю неподтверждённый/ }));
    fireEvent.change(screen.getByLabelText('Голос'), { target: { value: 'voice-two' } });
    expect(screen.getByLabelText('Референс')).not.toHaveProperty('value', '');
    fireEvent.click(screen.getByRole('checkbox', { name: /Разрешаю неподтверждённый/ }));
    expect(screen.getByLabelText('Референс')).toHaveProperty('value', '');
  });

  it('shows a catalog loading failure without exposing an empty selection', async () => {
    invoke.mockImplementation((command: string) =>
      command === 'get_speech_catalog_cmd'
        ? Promise.reject(new Error('Catalog unavailable'))
        : Promise.resolve([]),
    );
    render(<SpeechCatalog projectId="project-two" />);
    expect(await screen.findByRole('alert')).toHaveProperty(
      'textContent',
      expect.stringContaining('Catalog unavailable'),
    );
    expect(screen.queryByLabelText('Голос')).toBeNull();
  });

  it('keeps ready raw speech and assembly history accessible when the TTS catalog is unavailable', async () => {
    invoke.mockImplementation((command: string) => {
      if (command === 'get_speech_catalog_cmd')
        return Promise.reject(new Error('TTS runtime unavailable'));
      if (command === 'list_speech_runs_cmd')
        return Promise.resolve([
          {
            run_id: 'raw-ready',
            profile_id: 'fixture-profile',
            utterance_count: 1,
            chunk_count: 1,
            job: null,
            result: { ready: true },
          },
        ]);
      return Promise.resolve([]);
    });
    render(<SpeechCatalog projectId="project-three" />);
    await screen.findByText('TTS runtime unavailable');
    await screen.findByRole('option', { name: /raw-read/ });
    fireEvent.change(screen.getByLabelText('Готовый исходный синтез'), {
      target: { value: 'raw-ready' },
    });
    expect(screen.getByRole('button', { name: 'Измерить и показать тайминг' })).toHaveProperty(
      'disabled',
      false,
    );
    expect(screen.getByLabelText('Сборка итогового аудио')).not.toBeNull();
    expect(invoke).toHaveBeenCalledWith('list_audio_assemblies_cmd', {
      projectId: 'project-three',
    });
  });

  it('keeps text editing and preparation accessible when voice runtime is unavailable', async () => {
    invoke.mockImplementation((command: string) =>
      command === 'get_speech_catalog_cmd'
        ? Promise.reject(new Error('Voice runtime unavailable'))
        : Promise.resolve([]),
    );
    render(
      <SpeechCatalog
        projectId="project-three"
        stage="text"
        initialSource={{ kind: 'text', text: 'Независимый текст', translation_id: null }}
      />,
    );
    expect(
      await screen.findByRole('textbox', { name: 'Русский текст для озвучки' }),
    ).toHaveProperty('value', 'Независимый текст');
    expect(await screen.findByRole('alert')).toHaveProperty(
      'textContent',
      expect.stringContaining('Voice runtime unavailable'),
    );
    expect(screen.getByRole('button', { name: /Подготовить текст/ })).toBeTruthy();
  });
});
