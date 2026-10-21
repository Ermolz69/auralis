import type {
  SpeechCatalogSnapshot,
  SpeechVoiceOption,
} from '@/shared/api/contracts/speechCatalog';
type Props = {
  catalog: SpeechCatalogSnapshot;
  voices: SpeechVoiceOption[];
  profileId: string;
  voiceId: string;
  referenceId: string;
  disabled: boolean;
  chooseProfile: (id: string) => void;
  chooseVoice: (id: string) => void;
  chooseReference: (id: string) => void;
};
const select = 'mt-1 w-full rounded border border-border bg-surface p-2 text-text';
export function SpeechVoiceSettings(props: Props) {
  const {
    catalog,
    voices,
    profileId,
    voiceId,
    referenceId,
    disabled,
    chooseProfile,
    chooseVoice,
    chooseReference,
  } = props;
  const profile = catalog.profiles.find((p) => p.profile_id === profileId);
  const selected = voices.find((v) => v.voice_id === voiceId);
  const references =
    selected?.references.filter(
      (r) => r.engine_id === profile?.engine_id && r.emotion_id === 'neutral',
    ) ?? [];
  const engines = [...new Set(catalog.profiles.map((p) => p.engine_id))].filter((e) =>
    ['voxcpm2', 'espeech'].includes(e),
  );
  return (
    <fieldset className="space-y-3 rounded border border-border p-3" disabled={disabled}>
      <legend className="px-1 text-sm font-medium text-text">2. Голос и движок</legend>
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="text-xs text-muted">
          Движок
          <select
            aria-label="Движок"
            className={select}
            value={profile?.engine_id ?? ''}
            onChange={(event) => {
              const engine = event.target.value;
              const candidate =
                catalog.profiles.find((p) => p.profile_id === `native-plan-${engine}`) ??
                catalog.profiles.find((p) => p.engine_id === engine);
              if (candidate) chooseProfile(candidate.profile_id);
            }}
          >
            {engines.map((engine) => (
              <option key={engine} value={engine}>
                {engine === 'voxcpm2' ? 'VoxCPM2' : 'ESpeech'}
              </option>
            ))}
          </select>
        </label>
        <label className="text-xs text-muted">
          Голос
          <select
            aria-label="Голос"
            className={select}
            value={voiceId}
            onChange={(event) => chooseVoice(event.target.value)}
          >
            <option value="">Выберите голос</option>
            {voices.map((voice) => (
              <option key={voice.voice_id} value={voice.voice_id}>
                {voice.display_name}
              </option>
            ))}
          </select>
        </label>
      </div>
      {references.find((r) => r.reference_id === referenceId)?.qualification_status ===
      'qualified' ? (
        <p role="status" className="text-xs text-muted">
          Нейтральный образец подтверждён прослушиванием.
        </p>
      ) : (
        <p role="status" className="text-xs text-muted">
          Для этого голоса ещё нужен подтверждённый нейтральный образец.
        </p>
      )}
      <details className="text-xs text-muted">
        <summary className="cursor-pointer">Другие профили и образцы</summary>
        <div className="mt-2 grid gap-3 sm:grid-cols-2">
          <label>
            Профиль
            <select
              aria-label="Движок и профиль"
              className={select}
              value={profileId}
              onChange={(event) => chooseProfile(event.target.value)}
            >
              {catalog.profiles
                .filter((p) => engines.includes(p.engine_id))
                .map((p) => (
                  <option key={p.profile_id} value={p.profile_id}>
                    {p.profile_id}
                  </option>
                ))}
            </select>
          </label>
          <label>
            Образец
            <select
              aria-label="Референс"
              className={select}
              value={referenceId}
              onChange={(event) => chooseReference(event.target.value)}
            >
              <option value="">Выберите подтверждённый образец</option>
              {references.map((r) => (
                <option
                  key={r.reference_id}
                  value={r.reference_id}
                  disabled={r.qualification_status !== 'qualified'}
                >
                  {r.transcript || 'Нейтральный образец'}
                </option>
              ))}
            </select>
          </label>
        </div>
      </details>
    </fieldset>
  );
}
