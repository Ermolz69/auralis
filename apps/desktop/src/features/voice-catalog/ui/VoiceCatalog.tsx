import { Button } from '@/shared/ui/button';
import { useVoiceCatalog } from './useVoiceCatalog';

type Props = { projectId: string };

export function VoiceCatalog({ projectId }: Props) {
  const {
    voices,
    nextVoice,
    voice,
    playback,
    cast,
    roles,
    selections,
    roleId,
    roleName,
    busy,
    error,
    notice,
    setRoleId,
    setRoleName,
    loadMoreVoices,
    inspectVoice,
    inspectAudio,
    chooseReference,
    saveCast,
    recover,
  } = useVoiceCatalog(projectId);
  return (
    <section
      className="rounded-md border border-border bg-surface p-4"
      aria-label="Каталог голосов и ручное распределение ролей"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-sm font-semibold text-text">Голоса и роли</h2>
        <Button
          size="sm"
          onClick={() => void saveCast()}
          disabled={busy || Object.keys(selections).length === 0}
        >
          Сохранить cast
        </Button>
      </div>
      <p className="mt-2 text-xs text-muted">
        Роли задаются явно. Для назначения нужен образец с подтверждённым человеческим
        прослушиванием.
      </p>
      {error && (
        <div role="alert" className="mt-2 text-sm text-danger">
          {error}{' '}
          <Button size="sm" variant="ghost" onClick={() => void recover()}>
            Восстановить cast
          </Button>
        </div>
      )}
      {notice && (
        <p role="status" className="mt-2 text-sm text-muted">
          {notice}
        </p>
      )}
      <div className="mt-4 grid gap-4 lg:grid-cols-2">
        <div>
          <h3 className="text-sm font-medium">Каталог</h3>
          <ul className="mt-2 space-y-2">
            {voices.map((entry) => (
              <li key={entry.voice_id} className="rounded border border-border p-2 text-sm">
                <div className="flex items-center justify-between gap-2">
                  <span>
                    {entry.display_name ?? entry.voice_id}{' '}
                    <span className="text-muted">({entry.state})</span>
                  </span>
                  {entry.state === 'valid' && (
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => void inspectVoice(entry.voice_id)}
                    >
                      Открыть
                    </Button>
                  )}
                </div>
                {entry.state === 'valid' && (
                  <p className="text-xs text-muted">
                    {entry.reference_count} образцов · {entry.qualified_capabilities?.length ?? 0}{' '}
                    подтверждённых сочетаний
                  </p>
                )}
                {entry.error_code && <p className="text-xs text-danger">{entry.error_code}</p>}
              </li>
            ))}
          </ul>
          {nextVoice && (
            <Button size="sm" variant="ghost" onClick={() => void loadMoreVoices()} disabled={busy}>
              Ещё голоса
            </Button>
          )}
        </div>
        <div>
          <h3 className="text-sm font-medium">Роль</h3>
          <div className="mt-2 flex flex-wrap gap-2">
            <label className="text-xs">
              ID{' '}
              <input
                className="block rounded border border-border bg-bg p-1 text-sm"
                value={roleId}
                onChange={(event) => setRoleId(event.target.value)}
              />
            </label>
            <label className="text-xs">
              Название{' '}
              <input
                className="block rounded border border-border bg-bg p-1 text-sm"
                value={roleName}
                onChange={(event) => setRoleName(event.target.value)}
              />
            </label>
          </div>
          <p className="mt-2 text-xs text-muted">
            Активная ревизия: {cast?.revision_sha256.slice(0, 12) ?? 'нет'}
          </p>
          <ul className="mt-2 space-y-1 text-sm">
            {Object.entries(roles).map(([id, role]) => (
              <li key={id}>
                {role.display_name} ({id}): {role.voice_id ?? 'без голоса'}
                {selections[id] && ` · ${selections[id].engine_id}/${selections[id].emotion_id}`}
              </li>
            ))}
          </ul>
          {voice && (
            <div className="mt-4 rounded border border-border p-2">
              <h4 className="text-sm font-medium">{voice.display_name}</h4>
              <p className="text-xs text-muted">{voice.description}</p>
              <ul className="mt-2 space-y-2">
                {voice.references.map((reference) => (
                  <li
                    key={reference.reference_id}
                    className="rounded border border-muted p-2 text-xs"
                  >
                    <div>
                      {reference.reference_id} · {reference.engine_id}/{reference.emotion_id} ·{' '}
                      {reference.qualification_status}
                    </div>
                    <p className="mt-1">{reference.transcript}</p>
                    <div className="mt-2 flex gap-2">
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => void inspectAudio(reference)}
                      >
                        Слушать
                      </Button>
                      <Button
                        size="sm"
                        onClick={() => void chooseReference(reference)}
                        disabled={busy || reference.qualification_status !== 'qualified'}
                      >
                        Назначить роли
                      </Button>
                    </div>
                  </li>
                ))}
              </ul>
              {voice.next_after_reference && (
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => void inspectVoice(voice.voice_id, voice.next_after_reference)}
                  disabled={busy}
                >
                  Ещё образцы
                </Button>
              )}
              {playback && <audio className="mt-3 w-full" controls src={playback} />}
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
