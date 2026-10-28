import { useEffect, useMemo, useState } from 'react';
import type { SpeechCatalogSnapshot } from '@/shared/api/contracts/speechCatalog';
import { invoke } from '@/shared/api/tauri';

type Props = { projectId: string };

export function SpeechCatalog({ projectId }: Props) {
  const [catalog, setCatalog] = useState<SpeechCatalogSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [profileId, setProfileId] = useState('');
  const [voiceId, setVoiceId] = useState('');
  const [referenceId, setReferenceId] = useState('');

  useEffect(() => {
    let active = true;
    void invoke('get_speech_catalog_cmd', { projectId })
      .then((result) => {
        if (!active) return;
        setCatalog(result);
        setProfileId(result.profiles[0]?.profile_id ?? '');
        setError(null);
      })
      .catch((reason: unknown) => {
        if (active) setError(reason instanceof Error ? reason.message : String(reason));
      });
    return () => {
      active = false;
    };
  }, [projectId]);

  const profile = catalog?.profiles.find((item) => item.profile_id === profileId);
  const voices = useMemo(
    () =>
      catalog?.voices.filter((voice) =>
        voice.references.some((reference) => reference.engine_id === profile?.engine_id),
      ) ?? [],
    [catalog, profile?.engine_id],
  );
  const selectedVoice = voices.find((voice) => voice.voice_id === voiceId);
  const references =
    selectedVoice?.references.filter((reference) => reference.engine_id === profile?.engine_id) ??
    [];
  const selectedReference = references.find((reference) => reference.reference_id === referenceId);

  return (
    <section
      className="rounded-md border border-border bg-surface p-4"
      aria-label="Голоса синтеза речи"
    >
      <h2 className="text-sm font-semibold text-text">Синтез речи</h2>
      {error && (
        <p role="status" className="mt-2 text-xs text-muted">
          TTS недоступен: {error}
        </p>
      )}
      {catalog && (
        <div className="mt-3 grid gap-3 sm:grid-cols-3">
          <label className="text-xs text-muted">
            Движок и профиль
            <select
              className="mt-1 w-full rounded border border-border bg-surface p-2 text-text"
              value={profileId}
              onChange={(event) => {
                setProfileId(event.target.value);
                setVoiceId('');
                setReferenceId('');
              }}
            >
              {catalog.profiles.map((item) => (
                <option key={item.profile_id} value={item.profile_id}>
                  {item.engine_id} · {item.profile_id}
                </option>
              ))}
            </select>
          </label>
          <label className="text-xs text-muted">
            Голос
            <select
              className="mt-1 w-full rounded border border-border bg-surface p-2 text-text"
              value={voiceId}
              onChange={(event) => {
                setVoiceId(event.target.value);
                setReferenceId('');
              }}
            >
              <option value="">Выберите голос</option>
              {voices.map((voice) => (
                <option key={voice.voice_id} value={voice.voice_id}>
                  {voice.display_name}
                </option>
              ))}
            </select>
          </label>
          <label className="text-xs text-muted">
            Референс
            <select
              className="mt-1 w-full rounded border border-border bg-surface p-2 text-text"
              value={referenceId}
              onChange={(event) => setReferenceId(event.target.value)}
              disabled={!selectedVoice}
            >
              <option value="">Выберите референс</option>
              {references.map((reference) => (
                <option key={reference.reference_id} value={reference.reference_id}>
                  {reference.emotion_id} · {reference.reference_id}
                </option>
              ))}
            </select>
          </label>
        </div>
      )}
      {selectedReference && (
        <p className="mt-2 text-xs text-muted">
          {selectedReference.qualification_status === 'qualified'
            ? 'Референс подтверждён для выбранного движка.'
            : 'Голос выбран; проверка референса для синтеза ещё не завершена.'}
        </p>
      )}
    </section>
  );
}
