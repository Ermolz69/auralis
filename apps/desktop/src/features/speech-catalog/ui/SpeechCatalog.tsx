import { useEffect, useMemo, useState } from 'react';
import type { SpeechCatalogSnapshot } from '@/shared/api/contracts/speechCatalog';
import { describeCommandError } from '@/shared/api/contracts/error';
import { invoke } from '@/shared/api/tauri';
import { SpeechWorkflow } from './SpeechWorkflow';
import { SpeechVoiceSettings } from './SpeechVoiceSettings';
import { AudioAssemblyWorkspace } from './audio-assembly/AudioAssemblyWorkspace';
import { referenceAllowed } from '../model/referenceAllowed';
import type { SpeechInput } from '../model/speechDraft';

export function SpeechCatalog({ projectId, ...input }: { projectId: string } & SpeechInput) {
  const [catalog, setCatalog] = useState<SpeechCatalogSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [profileId, setProfileId] = useState('');
  const [voiceId, setVoiceId] = useState('');
  const [referenceId, setReferenceId] = useState('');
  const [revision, setRevision] = useState(0);
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState('');
  const [technical, setTechnical] = useState(false);
  useEffect(() => {
    let active = true;
    setCatalog(null);
    setTechnical(false);
    setError(null);
    void invoke('get_speech_catalog_cmd', { projectId })
      .then(async (result) => {
        if (!active) return;
        setCatalog(result);
        const first =
          result.profiles.find((p) => p.profile_id === 'native-plan-voxcpm2') ?? result.profiles[0];
        setProfileId(first?.profile_id ?? '');
        const voice = result.voices.find((v) =>
          v.references.some(
            (r) => r.engine_id === first?.engine_id && r.qualification_status === 'qualified',
          ),
        );
        setVoiceId(voice?.voice_id ?? '');
        setReferenceId(
          voice?.references.find(
            (r) =>
              r.engine_id === first?.engine_id &&
              r.emotion_id === 'neutral' &&
              r.qualification_status === 'qualified',
          )?.reference_id ??
            voice?.references.find(
              (r) => r.engine_id === first?.engine_id && r.qualification_status === 'qualified',
            )?.reference_id ??
            '',
        );
        const saved = await invoke('get_speech_voice_selection_cmd', { projectId });
        if (active && saved) {
          const savedProfile =
            result.profiles.find((p) => p.profile_id === `native-plan-${saved.engine_id}`) ??
            result.profiles.find((p) => p.engine_id === saved.engine_id);
          const savedVoice = result.voices.find((voice) => voice.voice_id === saved.voice_id);
          const savedReference = savedVoice?.references.find(
            (reference) =>
              reference.reference_id === saved.reference_id &&
              reference.engine_id === savedProfile?.engine_id &&
              reference.qualification_status === 'qualified',
          );
          if (savedProfile && savedReference) {
            setProfileId(savedProfile.profile_id);
            setVoiceId(saved.voice_id);
            setReferenceId(saved.reference_id);
          }
        }
      })
      .catch((reason) => {
        if (active) setError(describeCommandError(reason));
      });
    return () => {
      active = false;
    };
  }, [projectId, revision]);
  const profile = catalog?.profiles.find((p) => p.profile_id === profileId);
  const voices = useMemo(
    () =>
      catalog?.voices.filter((v) => v.references.some((r) => r.engine_id === profile?.engine_id)) ??
      [],
    [catalog, profile?.engine_id],
  );
  const chooseVoice = (id: string, engine = profile?.engine_id) => {
    setNotice('');
    setVoiceId(id);
    const references = catalog?.voices.find((voice) => voice.voice_id === id)?.references ?? [];
    setReferenceId(
      references.find(
        (reference) =>
          reference.engine_id === engine &&
          reference.emotion_id === 'neutral' &&
          referenceAllowed(reference, technical),
      )?.reference_id ??
        references.find(
          (reference) => reference.engine_id === engine && referenceAllowed(reference, technical),
        )?.reference_id ??
        '',
    );
  };
  const chooseProfile = (id: string) => {
    const next = catalog?.profiles.find((p) => p.profile_id === id);
    setProfileId(id);
    const voice = catalog?.voices.find((v) =>
      v.references.some((r) => r.engine_id === next?.engine_id && referenceAllowed(r, technical)),
    );
    chooseVoice(voice?.voice_id ?? '', next?.engine_id);
  };
  const saveVoice = async () => {
    setSaving(true);
    setError(null);
    setNotice('');
    try {
      await invoke('save_speech_voice_selection_cmd', {
        projectId,
        profileId,
        voiceId,
        referenceId,
        ...(technical ? { allowUnverifiedNeutral: true } : {}),
      });
      setNotice(
        technical
          ? 'Технический выбор сохранён; качество образца не подтверждено.'
          : 'Голос проекта сохранён и проверен.',
      );
    } catch (reason) {
      setError(describeCommandError(reason));
    } finally {
      setSaving(false);
    }
  };
  return (
    <section
      className="rounded-md border border-border bg-surface p-4"
      aria-label="Голоса синтеза речи"
    >
      <h2 className="text-base font-semibold text-text">
        {input.stage === 'text'
          ? 'Текст и подготовка'
          : input.stage === 'voices'
            ? 'Голоса и образцы'
            : input.stage === 'speech'
              ? 'Синтез речи'
              : 'Текст и озвучка'}
      </h2>
      <p className="mt-1 text-xs text-muted">
        Добавьте русский текст или субтитры, проверьте подготовку и выберите голос.
      </p>
      {error && (
        <div role="alert" className="mt-3 text-sm text-danger">
          {error}
          <button className="ml-2 underline" onClick={() => setRevision((value) => value + 1)}>
            Повторить подключение
          </button>
        </div>
      )}
      {!catalog && !error && (
        <p role="status" className="mt-3 text-xs text-muted">
          Проверяем проект и библиотеку голосов…
        </p>
      )}
      {(catalog || input.stage === 'text') && (
        <>
          {catalog && (
            <p
              title="Проект и образцы проверены через PUZA CONTRACT"
              className="mt-2 text-xs text-muted"
            >
              Проект проверен · библиотека голосов подключена
            </p>
          )}
          <SpeechWorkflow
            {...input}
            projectId={projectId}
            profileId={profileId}
            voiceId={voiceId}
            referenceId={referenceId}
            voices={voices}
            engineId={profile?.engine_id ?? ''}
            allowUnverifiedNeutral={technical}
            renderVoiceSettings={(disabled) =>
              catalog && (
                <>
                  <SpeechVoiceSettings
                    catalog={catalog}
                    voices={voices}
                    profileId={profileId}
                    voiceId={voiceId}
                    referenceId={referenceId}
                    disabled={disabled || saving}
                    technical={technical}
                    setTechnical={(value) => {
                      setTechnical(value);
                      setReferenceId('');
                      setVoiceId('');
                    }}
                    chooseProfile={chooseProfile}
                    chooseVoice={chooseVoice}
                    chooseReference={setReferenceId}
                  />
                  <button
                    className="rounded border border-border px-3 py-2 text-xs text-text disabled:opacity-40"
                    disabled={disabled || saving || !referenceId}
                    onClick={() => void saveVoice()}
                  >
                    Сохранить голос проекта
                  </button>
                  {notice && (
                    <p role="status" className="text-xs text-muted">
                      {notice}
                    </p>
                  )}
                </>
              )
            }
          />
        </>
      )}
      <AudioAssemblyWorkspace key={projectId} projectId={projectId} />
    </section>
  );
}
