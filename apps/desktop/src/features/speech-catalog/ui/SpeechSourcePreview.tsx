import type { SpeechAssignment, SpeechSourceSnapshot } from '@/shared/api/contracts/speech';
import type { SpeechVoiceOption } from '@/shared/api/contracts/speechCatalog';
import { useMemo } from 'react';
import { SpeechPreviewPage } from './SpeechPreviewPage';
type Props = {
  source: SpeechSourceSnapshot | null;
  voices: SpeechVoiceOption[];
  engineId: string;
  assignments: SpeechAssignment[];
  assign: (scope: 'role' | 'cue', target: string, voice: string, reference?: string) => void;
};
export function SpeechSourcePreview(props: Props) {
  const { source } = props;
  if (!source) return null;
  return (
    <SourcePreview
      key={`${source.kind}:${source.source_id}:${source.revision}:${source.sha256}`}
      {...props}
      source={source}
    />
  );
}

function SourcePreview({
  source,
  voices,
  engineId,
  assignments,
  assign,
}: Props & { source: SpeechSourceSnapshot }) {
  const roles = useMemo(() => [...new Set(source.cues.map((cue) => cue.role))], [source.cues]);
  const byTarget = useMemo(
    () => new Map(assignments.map((a) => [`${a.scope}\0${a.target}`, a])),
    [assignments],
  );
  const assignmentSelect = (scope: 'role' | 'cue', target: string) => {
    const current = byTarget.get(`${scope}\0${target}`);
    const references =
      voices
        .find((voice) => voice.voice_id === current?.voice_id)
        ?.references.filter(
          (reference) =>
            reference.engine_id === engineId && reference.qualification_status === 'qualified',
        ) ?? [];
    return (
      <span className="flex flex-wrap gap-1">
        <select
          aria-label={`Голос ${scope === 'role' ? 'роли' : 'реплики'} ${target}`}
          className="rounded border border-border bg-surface p-1 text-text"
          value={current?.voice_id ?? ''}
          onChange={(e) => assign(scope, target, e.target.value)}
        >
          <option value="">Общий голос</option>
          {voices
            .filter((voice) =>
              voice.references.some(
                (reference) =>
                  reference.engine_id === engineId &&
                  reference.qualification_status === 'qualified',
              ),
            )
            .map((voice) => (
              <option key={voice.voice_id} value={voice.voice_id}>
                {voice.display_name}
              </option>
            ))}
        </select>
        {current && (
          <select
            aria-label={`Эмоция ${scope === 'role' ? 'роли' : 'реплики'} ${target}`}
            className="rounded border border-border bg-surface p-1 text-text"
            value={current.reference_id}
            onChange={(e) => assign(scope, target, current.voice_id, e.target.value)}
          >
            {references.map((reference) => (
              <option key={reference.reference_id} value={reference.reference_id}>
                {reference.emotion_id}
              </option>
            ))}
          </select>
        )}
      </span>
    );
  };
  return (
    <>
      {source && (
        <div className="max-h-64 space-y-2 overflow-auto rounded border border-border p-2 text-text">
          {source.cues.length > 0 ? (
            <>
              <SpeechPreviewPage
                label="Роли"
                items={roles}
                renderItem={(role) => (
                  <label key={role} className="flex justify-between gap-2">
                    Роль: {role}
                    {assignmentSelect('role', role)}
                  </label>
                )}
              />
              <SpeechPreviewPage
                label="Реплики"
                items={source.cues}
                renderItem={(cue, q) => (
                  <div key={cue.id} className="border-t border-border pt-2">
                    <div className="flex justify-between gap-2">
                      <span>
                        {q + 1} · {(cue.start_ms / 1000).toFixed(2)}–
                        {(cue.end_ms / 1000).toFixed(2)} с
                      </span>
                      {assignmentSelect('cue', cue.id)}
                    </div>
                    <p className="mt-1 whitespace-pre-wrap">{cue.text}</p>
                  </div>
                )}
              />
            </>
          ) : (
            <p className="whitespace-pre-wrap">{source.text}</p>
          )}
        </div>
      )}
    </>
  );
}
